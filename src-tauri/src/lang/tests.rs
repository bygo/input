use super::*;

#[test]
fn english_selected_goes_to_chinese() {
    assert_eq!(route("Work on it for 14 hours.", "中文", "英语"), "中文");
    assert_eq!(route("Have you eaten yet today?", "中文", "英语"), "中文");
}

#[test]
fn chinese_selected_goes_to_english() {
    assert_eq!(route("你好，今天吃饭了吗", "中文", "英语"), "英语");
}

#[test]
fn non_english_goes_to_english() {
    assert_eq!(route("こんにちは", "中文", "英语"), "英语");
    assert_eq!(route("jsonformatter 都几万人做", "中文", "英语"), "英语");
}

#[test]
fn pure_english_detection() {
    assert!(is_pure_english("Have you eaten yet today?"));
    assert!(!is_pure_english("jsonformatter 都几万人做"));
    assert!(!is_pure_english("你好"));
    assert!(!is_pure_english("123 !!!"));
}

#[test]
fn selection_route_pure_english_only() {
    assert_eq!(route("Hello world", "中文", "英语"), "中文");
    assert_eq!(route("jsonformatter 都几万人做", "中文", "英语"), "英语");
    assert_eq!(route("你好", "中文", "英语"), "英语");
    assert!(is_pure_english("Hello world"));
    assert!(!is_pure_english("jsonformatter 都几万人做"));
}

#[test]
fn chinese_rejects_english_leftovers() {
    assert!(!fits_target(
        "overnight没有出现重大新冲击，但原油仍在反应。",
        "中文"
    ));
    assert!(!fits_target(
        "隔夜没有重大新冲击，但 crude 仍在反应。",
        "中文"
    ));
    assert!(fits_target(
        "隔夜没有出现重大新冲击，但原油仍在对持续的美伊局势做出反应。",
        "中文"
    ));
    assert!(fits_target("可以，OK。", "中文"));
    assert!(fits_target(
        "Fing.0.00毫秒\nAPM：244\n8\nTTiner 00:17:15\n时间：\n17:07",
        "中文"
    ));
    assert!(!fits_target(
        "Fing.0.00ms\nAPM: 244\nTime：\n17:07",
        "中文"
    ));
}

#[test]
fn english_rejects_chinese_leftovers() {
    assert!(!fits_target("Hello 你好 world", "英语"));
    assert!(!fits_target("How to read books呢", "英语"));
    assert!(!fits_target(
        "Don't force me\nKaho 0:2 Focus\n辰漫\n浪漫 0:2 Happy",
        "英语"
    ));
    assert!(fits_target("Have you eaten yet today?", "英语"));
    assert!(fits_target("How to read books?", "英语"));
    assert!(fits_target(
        "Chen Man\nRomance 0:2 Happy\nKaho 0:2 Focus",
        "英语"
    ));
}
