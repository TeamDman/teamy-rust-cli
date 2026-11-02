/// Escape untrusted terminal controls and bidirectional formatting characters.
#[must_use]
pub fn escape_terminal_text(input: &str) -> String {
    let mut output = String::new();
    for ch in input.chars() {
        if (ch.is_control() && ch != '\n' && ch != '\t')
            || matches!(ch, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        {
            output.extend(ch.escape_unicode());
        } else {
            output.push(ch);
        }
    }
    output
}
