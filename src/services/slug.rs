use slug::slugify;

pub fn generate_slug(title: &str) -> String {
    slugify(title)
}

pub fn validate_slug(slug: &str) -> bool {
    if slug.is_empty() || slug.len() > 200 {
        return false;
    }
    slug.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}
