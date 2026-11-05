
#[test]
fn str_key_derives_agree_across_the_abi() {
    use alm_rt::rt::{alm_str_intern, alm_str_key, is_str_key, str_key_of_bytes};

    let mut words: Vec<Vec<u8>> = Vec::new();
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.:-";
    let mut rng = Rng::new(0xAB1_5EED);
    for _ in 0..1500 {
        let len = 1 + rng.below(24);
        let word: Vec<u8> = (0..len)
            .map(|_| ALPHABET[rng.below(ALPHABET.len())])
            .collect();
        words.push(word);
    }
    let mut corpus_literals = 0usize;
    for source in corpus_sources().into_iter().map(|(_, s)| s) {
        let bytes = source.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'"' {
                let mut j = i + 1;
                while j < bytes.len() && bytes[j] != b'"' && bytes[j] != b'\n' {
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b'"' && j > i + 1 {
                    words.push(bytes[i + 1..j].to_vec());
                    corpus_literals += 1;
                }
                i = j + 1;
            } else {
                i += 1;
            }
        }
    }
    assert!(
        corpus_literals >= 10,
        "extracted only {corpus_literals} corpus string literals — cases/ vanished?"
    );

    for word in &words {
        let interned = unsafe { alm_str_intern(word.as_ptr(), word.len()) };
        assert!(
            !interned.is_null(),
            "the C pool refused to intern {:?} (malloc failure?)",
            String::from_utf8_lossy(word)
        );
        let c_derive = unsafe { alm_str_key(interned) };
        let rust_derive = str_key_of_bytes(word);
        assert_eq!(
            c_derive,
            rust_derive,
            "C pool prefix key and Rust derive disagree on {:?} — the frozen ABI broke",
            String::from_utf8_lossy(word)
        );
        assert!(
            is_str_key(c_derive),
            "key for {:?} fell outside the reserved str-key range",
            String::from_utf8_lossy(word)
        );
    }
    eprintln!(
        "str-key ABI coverage: {} strings ({} corpus literals) through both derives",
        words.len(),
        corpus_literals
    );
}
