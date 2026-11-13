//! Public contact metadata in the Solana security.txt V1 wire format.
//! This static data does not change instruction handlers or account layouts.
//! Format: https://github.com/neodyme-labs/solana-security-txt

#[cfg_attr(target_arch = "bpf", link_section = ".security.txt")]
#[used]
#[no_mangle]
pub static SECURITY_TXT: &str = concat!(
    "=======BEGIN SECURITY.TXT V1=======\0",
    "name\0Heisto\0",
    "project_url\0https://heisto.xyz\0",
    "contacts\0email:support@heisto.xyz\0",
    "policy\0https://github.com/heisto-dev/heisto-contracts/blob/main/SECURITY.md\0",
    "preferred_languages\0en\0",
    "source_code\0https://github.com/heisto-dev/heisto-contracts\0",
    "=======END SECURITY.TXT V1=======\0",
);

#[cfg(test)]
mod tests {
    use super::SECURITY_TXT;

    #[test]
    fn security_contact_uses_supported_wire_format() {
        let body = SECURITY_TXT
            .strip_prefix("=======BEGIN SECURITY.TXT V1=======\0")
            .expect("missing V1 begin marker")
            .strip_suffix("=======END SECURITY.TXT V1=======\0")
            .expect("missing V1 end marker");
        let fields: Vec<&str> = body.trim_end_matches('\0').split('\0').collect();
        assert_eq!(fields.len() % 2, 0);
        let entries: std::collections::BTreeMap<_, _> =
            fields.chunks_exact(2).map(|pair| (pair[0], pair[1])).collect();
        assert_eq!(entries.len(), 6);
        assert_eq!(entries["name"], "Heisto");
        assert_eq!(entries["contacts"], "email:support@heisto.xyz");
        assert_eq!(entries["project_url"], "https://heisto.xyz");
        assert_eq!(entries["preferred_languages"], "en");
        assert!(entries["policy"].ends_with("/SECURITY.md"));
    }
}
