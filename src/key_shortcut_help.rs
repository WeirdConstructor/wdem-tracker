pub fn get_shortcut_help_page(page: usize) -> String {
match page {
1 => String::from(r#"
[Step] Mode:

    When entering the mode the step is reset to 1, from that
    you can change it with these keys:

    0               - Multiply by 10
    1 - 9           - Add a value (1 to 9).
    any other key   - Go back to [Normal] mode.

[File] Mode:
    w               - Write contents of trackers and input values of
                      signal ops to `tracker.json` file.
    r               - Read contents of trackers and input values from
                      `tracker.json` again.

[Interpolation] Mode:
    s               - Step (no interpolation)
    l               - Linear interpolation
    e               - Exponential interpolation
    t               - Smoothstep interpolation
"#),
2 => String::from(r#"
[Note] Mode:

    Remember: In [Normal] mode you can always press the Alt key
    and a key from the [Note] mode to enter a note on the fly.

    + / -           - Go an octave up/down
    yxcvbnm         - Octave+0 White keys from C to B
    sdghj           - Octave+0 Black keys from C# to A#

    qwertzu         - Octave+1 White keys from C to B
    23567           - Octave+1 Black keys from C# to A#

    iop             - Octave+2 White keys from C to E
    90              - Octave+2 Black keys from C# to D#

[ScrollOps] Mode:
    h / j / k / l   - Scroll the signal groups / operators

[A] / [B] Mode:
    0-9 / A-F / a-f - Enter 2 hex digits
"#),
_ => String::from(
r#"
WDem Tracker - Keyboard Reference
=================================
- Hit ESC to get back.
- Space/PageDown for next page.
- Backspace/PageUp for previous page.

[Normal] Mode:
    h / l           - Move cursor to left/right track.
    j / k           - Step cursor down/up a row.
    Shift + j / k   - Move cursor down/up exactly 1 row (regardless of the
                      step size).
    s               - Go to `Step` mode for setting the step size.
    x               - Delete contents of cursor cell.
    f               - Go to `File` mode, for writing/reading the
                      current contents of the tracks and input signals.
    y               - Refresh signal operator from background thread.
    i               - Go to `Interpolation` mode for setting the interpolation
                      of the current track.
    ' ' (space)     - Pause/Unpause the tracker.
    '#'             - Go to `Note` mode for entering notes by keyboard.
                      For quickly entering notes hit the Alt key and the
                      notes on the keyboard according to `Note` mode.
    'o'             - Go to `ScrollOps` mode for scrolling the displayed
                      signal groups and operators using the h/j/k/l keys.
    n / m           - Stop the tracker and move the play cursor up/down a row.
    a               - Go to `A` mode for entering the A 8-bit hex value.
    b               - Go to `B` mode for entering the B 8-bit hex value.
    - / . / 0-9     - For entering a value, just start typing the value
                      and hit Return or some other key.
"#),
}
}
