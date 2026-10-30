//! The crate names no consumer: no site or blog words in `src/`.

#[test]
fn the_crate_names_no_consumer() {
    const FORBIDDEN: &[&str] = &[
        "site", "page", "post", "posts", "blog", "feed", "gallery", "highbay",
    ];
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offences = Vec::new();
    let mut files = Vec::new();
    let mut dirs = vec![src];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
            } else {
                files.push(path);
            }
        }
    }
    assert!(files.len() > 10, "the walk found the sources");
    for path in files {
        let text = std::fs::read_to_string(&path).unwrap().to_lowercase();
        for (n, line) in text.lines().enumerate() {
            for word in line.split(|c: char| !c.is_ascii_alphanumeric()) {
                if FORBIDDEN.contains(&word) {
                    offences.push(format!("{}:{}: {word}", path.display(), n + 1));
                }
            }
        }
    }
    assert!(
        offences.is_empty(),
        "consumer words in src/:\n{}",
        offences.join("\n")
    );
}
