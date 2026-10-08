/// Actor para auditoría mientras no exista login: el nombre declarado por
/// el cliente, o `None` si no vino o está en blanco.
pub fn declared_actor(declared: &Option<String>) -> Option<String> {
    declared
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declared_actor_trims_and_drops_blank_names() {
        assert_eq!(declared_actor(&None), None);
        assert_eq!(declared_actor(&Some("   ".to_string())), None);
        assert_eq!(
            declared_actor(&Some("  Ana ".to_string())),
            Some("Ana".to_string())
        );
    }
}
