use super::core::LayoutVerdict;
use crate::ast::{Expr, NodeId, Stmt};
use crate::diag::Diagnostic;
use crate::shape::Ty;
use alm_rt::{signal, trace};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, PartialEq, Debug)]
pub enum Keep {
    Name(String),
    #[allow(dead_code)]
    Ghost(usize),
}

#[derive(Clone)]
pub struct DoExitFree {
    #[allow(dead_code)]
    pub site: usize,
    pub carrier: String,
    pub keeps: Vec<Keep>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TagSrc {
    If(NodeId),
    Call(NodeId),
}

#[derive(Clone)]
pub struct ArmFreeEntry {
    pub carrier: String,
    pub tag: TagSrc,
    pub nothing_on: bool,
    pub keeps: Vec<Keep>,
}

#[derive(Clone)]
pub struct FnDef {
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
    pub ret: Ty,
    pub signature: Option<Vec<Ty>>,
}

#[derive(Clone)]
pub enum FfiVerb {
    Load,
    ArgKind,
    Cdef,
    CoroCreate,
    CoroResume,
    CoroYield,
    CoroStatus,
    CoroClose,
    Drive { base: String, sym: String },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyLane {
    Int,
    Str,
}

impl FnDef {
    pub fn body(&self) -> &[Stmt] {
        &self.body
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceId(pub u32);

impl InstanceId {
    pub const ROOT: InstanceId = InstanceId(0);
}

#[derive(Clone, Debug)]
pub struct InstanceInfo {
    #[allow(dead_code)]
    pub call: Option<NodeId>,
    pub fn_node: Option<NodeId>,
    #[allow(dead_code)]
    pub params: Vec<String>,
    #[allow(dead_code)]
    pub body: Vec<Stmt>,
    #[allow(dead_code)]
    pub frame_base: usize,
    pub break_base: usize,
    pub env_face: Option<BTreeMap<String, Ty>>,
    pub ret: Option<Ty>,
    pub children: BTreeMap<NodeId, BTreeSet<InstanceId>>,
    pub creation_envs: BTreeMap<NodeId, NodeId>,
    pub closure_calls: BTreeSet<NodeId>,
    pub index_key_lanes: BTreeMap<NodeId, KeyLane>,
    pub access_elems: BTreeMap<NodeId, Ty>,
}

#[derive(Clone)]
pub struct FillPlan {
    pub fills: BTreeSet<String>,
    pub nested_fills: BTreeSet<String>,
}

#[derive(Clone)]
pub struct ShapeFacts {
    pub sites: BTreeMap<NodeId, usize>,
    pub elems: Vec<Ty>,
    pub boundary_elem: Option<Ty>,
    #[allow(dead_code)]
    pub layouts: Vec<LayoutVerdict>,
    pub free_sites: BTreeSet<NodeId>,
    pub free_keeps: BTreeMap<NodeId, Vec<Keep>>,
    pub do_exit_frees: BTreeMap<(NodeId, u8), Vec<DoExitFree>>,
    pub root_exit_frees: Vec<DoExitFree>,
    pub stmt_temp_frees: BTreeMap<NodeId, Vec<usize>>,
    pub cond_temp_frees: BTreeMap<NodeId, Vec<usize>>,
    pub arm_temp_frees: BTreeMap<NodeId, Vec<usize>>,
    pub ret_path_frees: BTreeMap<NodeId, Vec<DoExitFree>>,
    pub break_path_frees: BTreeMap<NodeId, Vec<DoExitFree>>,
    pub ret_path_keep_frees: BTreeMap<NodeId, Vec<DoExitFree>>,
    pub ret_path_keep_temps: BTreeMap<NodeId, Vec<usize>>,
    pub rebind_frees: BTreeMap<NodeId, BTreeMap<String, Vec<Keep>>>,
    pub join_tags: BTreeMap<NodeId, bool>,
    pub call_tags: BTreeMap<NodeId, Vec<bool>>,
    pub store_ghost_keeps: BTreeMap<NodeId, Vec<usize>>,
    pub ctor_ghost_keeps: BTreeMap<NodeId, Vec<(usize, usize)>>,
    pub arm_free_sites: BTreeMap<NodeId, Vec<ArmFreeEntry>>,
    pub arm_rebind_frees: BTreeMap<NodeId, Vec<ArmFreeEntry>>,
    pub arm_do_exit_frees: BTreeMap<(NodeId, u8), Vec<ArmFreeEntry>>,
    pub arm_root_exit_frees: Vec<ArmFreeEntry>,
    pub move_poisons: BTreeMap<NodeId, BTreeSet<String>>,
    pub name_dense: BTreeMap<(String, usize), bool>,
    pub fill_loops: BTreeMap<NodeId, FillPlan>,
    pub stored_ctor_parents: BTreeSet<usize>,
    pub substitutions: BTreeMap<usize, Ty>,
    pub local_types: BTreeMap<(NodeId, InstanceId, String), Ty>,
    pub row_reads: BTreeMap<usize, (String, usize)>,
    pub cell_children: BTreeMap<usize, BTreeSet<usize>>,
    pub fn_scope_keys: BTreeMap<NodeId, (NodeId, u8)>,
    pub fn_touched: BTreeMap<NodeId, BTreeSet<String>>,
    pub fn_defs: BTreeMap<NodeId, FnDef>,
    pub env_faces: BTreeMap<NodeId, BTreeMap<String, Ty>>,
    pub closure_fields: BTreeMap<(usize, String), NodeId>,
    pub call_fns: BTreeMap<NodeId, BTreeSet<NodeId>>,
    pub ffi_verbs: BTreeMap<NodeId, FfiVerb>,
    pub instances: BTreeMap<InstanceId, InstanceInfo>,
    pub ctor_lines: Vec<usize>,
    #[allow(dead_code)]
    pub adopt_lines: BTreeMap<String, usize>,
    pub diagnostics: Vec<Diagnostic>,
}

impl ShapeFacts {
    pub fn elem_of(&self, ctor: &Expr) -> Ty {
        let id = self.sites[&ctor.id()];
        let elem = self.elems[id].clone();
        self.resolve_elem_type(elem)
    }

    fn resolve_elem_type(&self, ty: Ty) -> Ty {
        match &ty {
            Ty::Unknown(id) => match self.substitutions.get(id) {
                Some(resolved) => self.resolve_elem_type(resolved.clone()),
                None => ty,
            },
            Ty::Tbl(inner) => {
                let resolved = self.resolve_elem_type((**inner).clone());
                Ty::Tbl(Box::new(resolved))
            }
            _ => ty,
        }
    }

    pub fn is_dense_at_depth(&self, name: &str, depth: usize) -> bool {
        self.name_dense
            .get(&(name.to_string(), depth))
            .copied()
            .unwrap_or(false)
    }

    pub fn check_row_reads(&mut self) {
        let reads: Vec<(usize, String, usize)> = self
            .row_reads
            .iter()
            .map(|(&base, (name, depth))| (base, name.clone(), *depth))
            .collect();
        let mut reported: BTreeSet<usize> = BTreeSet::new();
        for (base, name, depth) in reads {
            let mut level: BTreeSet<usize> =
                self.cell_children.get(&base).cloned().unwrap_or_default();
            for _pierce in 2..=depth {
                let mut next: BTreeSet<usize> = BTreeSet::new();
                for row in level {
                    if reported.insert(row)
                        && matches!(
                            self.resolve_elem_type(self.elems[row].clone()),
                            Ty::Unknown(_)
                        )
                    {
                        signal!(trace::TRACE_ANALYZE_MISSING_VALUE);
                        let line = self.ctor_lines[row];
                        self.diagnostics.push(
                            Diagnostic::ty(format!(
                                "a row of '{name}' is read before it is \
                                 ever given a value (table site #{row})"
                            ))
                            .with_site(Some(line)),
                        );
                    }
                    if let Some(kids) = self.cell_children.get(&row) {
                        next.extend(kids.iter().copied());
                    }
                }
                level = next;
            }
        }
    }

    pub fn is_free(&self, stmt: &Stmt) -> bool {
        self.free_sites.contains(&stmt.id())
    }
}
