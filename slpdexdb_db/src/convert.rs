pub fn pg_safe_string(string: String) -> String {
    if !string.contains("\0") {
        return string;
    }
    string.chars().filter(|char| char != &'\0').collect::<String>()
}
