use super::*;

#[test]
fn normalizes_command_or_control_defaults() {
    let got = normalize_shortcut("CommandOrControl+G").unwrap();
    assert!(got == "Command+G" || got == "Control+G", "{got}");
    let got = normalize_shortcut("CommandOrControl+Shift+G").unwrap();
    assert!(
        got == "Command+Shift+G" || got == "Control+Shift+G",
        "{got}"
    );
    assert_eq!(normalize_shortcut("Alt+A").unwrap(), "Alt+A");
}

#[test]
fn normalizes_command_and_control_separately() {
    assert_eq!(normalize_shortcut("Command+A").unwrap(), "Command+A");
    assert_eq!(normalize_shortcut("Control+A").unwrap(), "Control+A");
    assert!(!same_shortcut("Command+A", "Control+A"));
}

#[test]
fn allows_function_keys() {
    assert_eq!(normalize_shortcut("F8").unwrap(), "F8");
    assert_eq!(normalize_shortcut("Shift+F3").unwrap(), "Shift+F3");
}

#[test]
fn rejects_bare_key() {
    assert!(normalize_shortcut("G").is_err());
    assert!(normalize_shortcut("Shift+G").is_err());
}

#[test]
fn treats_cmd_g_as_same_as_command_or_control() {
    assert!(same_shortcut("CommandOrControl+G", "Command+G"));
    assert!(!same_shortcut("Alt+A", "Alt+D"));
}
