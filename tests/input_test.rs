use cc_switch_tui::app::state::InputState;

#[test]
fn test_input_new() {
    let input = InputState::new("hello".to_string());
    assert_eq!(input.value, "hello");
    assert_eq!(input.cursor, 5);
}

#[test]
fn test_insert_char() {
    let mut input = InputState::new("helo".to_string());
    input.move_left();
    input.insert_char('l');
    assert_eq!(input.value, "hello");
    assert_eq!(input.cursor, 4);
}

#[test]
fn test_backspace() {
    let mut input = InputState::new("hello".to_string());
    input.backspace();
    assert_eq!(input.value, "hell");
    assert_eq!(input.cursor, 4);
}

#[test]
fn test_move_left_right() {
    let mut input = InputState::new("hi".to_string());
    input.move_left();
    assert_eq!(input.cursor, 1);
    input.move_left();
    assert_eq!(input.cursor, 0);
    input.move_right();
    assert_eq!(input.cursor, 1);
}

#[test]
fn test_unicode_insert_and_backspace() {
    let mut input = InputState::new("中".to_string());
    input.insert_char('文');
    assert_eq!(input.value, "中文");
    assert_eq!(input.cursor, 2);
    input.backspace();
    assert_eq!(input.value, "中");
    assert_eq!(input.cursor, 1);
}
