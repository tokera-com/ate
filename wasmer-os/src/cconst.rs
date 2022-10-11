use super::tty::Tty;

pub struct ConsoleConst {}

impl ConsoleConst {
    pub const TERM_KEY_ENTER: u32 = 13;
    pub const TERM_KEY_BACKSPACE: u32 = 8;
    pub const TERM_KEY_INSERT: u32 = 45;
    pub const TERM_KEY_DEL: u32 = 46;
    pub const TERM_KEY_TAB: u32 = 9;
    pub const TERM_KEY_HOME: u32 = 36;
    pub const TERM_KEY_END: u32 = 35;
    pub const TERM_KEY_PAGE_UP: u32 = 33;
    pub const TERM_KEY_PAGE_DOWN: u32 = 34;
    pub const TERM_KEY_LEFT_ARROW: u32 = 37;
    pub const TERM_KEY_UP_ARROW: u32 = 38;
    pub const TERM_KEY_RIGHT_ARROW: u32 = 39;
    pub const TERM_KEY_DOWN_ARROW: u32 = 40;
    pub const TERM_KEY_C: u32 = 67;
    pub const TERM_KEY_L: u32 = 76;
    pub const TERM_KEY_F1: u32 = 112;
    pub const TERM_KEY_F2: u32 = 113;
    pub const TERM_KEY_F3: u32 = 114;
    pub const TERM_KEY_F4: u32 = 115;
    pub const TERM_KEY_F5: u32 = 116;
    pub const TERM_KEY_F6: u32 = 117;
    pub const TERM_KEY_F7: u32 = 118;
    pub const TERM_KEY_F8: u32 = 119;
    pub const TERM_KEY_F9: u32 = 120;
    pub const TERM_KEY_F10: u32 = 121;
    pub const TERM_KEY_F11: u32 = 122;
    pub const TERM_KEY_F12: u32 = 123;
}

impl Tty {
    pub const TERM_CURSOR_UP: &'static str = "\x1b[A";
    pub const TERM_CURSOR_DOWN: &'static str = "\x1b[B";
    pub const TERM_CURSOR_RIGHT: &'static str = "\x1b[C";
    pub const TERM_CURSOR_LEFT: &'static str = "\x1b[D";

    pub const TERM_DELETE_LINE: &'static str = "\x1b[2K\r";
    pub const TERM_DELETE_RIGHT: &'static str = "\x1b[0K\r";
    pub const TERM_DELETE_LEFT: &'static str = "\x1b[1K\r";
    pub const TERM_DELETE_BELOW: &'static str = "\x1b[0J\r";
    pub const TERM_DELETE_ABOVE: &'static str = "\x1b[1J\r";
    pub const TERM_DELETE_ALL: &'static str = "\x1b[2J\r";
    pub const TERM_DELETE_SAVED: &'static str = "\x1b[3J\r";

    pub const TERM_CURSOR_SAVE: &'static str = "\x1b[s";
    pub const TERM_CURSOR_RESTORE: &'static str = "\x1b[u";

    pub const TERM_WRAPAROUND: &'static str = "\x1b[?7h";
    pub const TERM_REVERSE_WRAPAROUND: &'static str = "\x1b[?45h";

    pub const TERM_NO_WRAPAROUND: &'static str = "\x1b[?7l";
    pub const TERM_NO_REVERSE_WRAPAROUND: &'static str = "\x1b[?45l";

    pub const COL_BOLD: &'static str = "\x1B[34;1m";
    pub const COL_RESET: &'static str = "\x1B[0m";
    pub const COL_BLACK: &'static str = "\x1B[0;30m";
    pub const COL_GRAY: &'static str = "\x1B[1;30m";
    pub const COL_RED: &'static str = "\x1B[0;31m";
    pub const COL_LIGHT_RED: &'static str = "\x1B[1;31m";
    pub const COL_GREEN: &'static str = "\x1B[0;32m";
    pub const COL_LIGHT_GREEN: &'static str = "\x1B[1;32m";
    pub const COL_BROWN: &'static str = "\x1B[0;33m";
    pub const COL_YELLOW: &'static str = "\x1B[1;33m";
    pub const COL_BLUE: &'static str = "\x1B[0;34m";
    pub const COL_LIGHT_BLUE: &'static str = "\x1B[1;34m";
    pub const COL_PURPLE: &'static str = "\x1B[0;35m";
    pub const COL_LIGHT_PURPLE: &'static str = "\x1B[1;35m";
    pub const COL_CYAN: &'static str = "\x1B[0;36m";
    pub const COL_LIGHT_CYAN: &'static str = "\x1B[1;36m";
    pub const COL_LIGHT_GRAY: &'static str = "\x1B[0;37m";
    pub const COL_WHITE: &'static str = "\x1B[1;37m";

    
    pub const WELCOME: &'static str = r#"\x1B[1;34m██╗    ██╗ █████╗ ███████╗███╗   ███╗███████╗██████╗    ███████╗██╗  ██╗
██║    ██║██╔══██╗██╔════╝████╗ ████║██╔════╝██╔══██╗   ██╔════╝██║  ██║
██║ █╗ ██║███████║███████╗██╔████╔██║█████╗  ██████╔╝   ███████╗███████║
██║███╗██║██╔══██║╚════██║██║╚██╔╝██║██╔══╝  ██╔══██╗   ╚════██║██╔══██║
╚███╔███╔╝██║  ██║███████║██║ ╚═╝ ██║███████╗██║  ██║██╗███████║██║  ██║
 ╚══╝╚══╝ ╚═╝  ╚═╝╚══════╝╚═╝     ╚═╝╚══════╝╚═╝  ╚═╝╚═╝╚══════╝╚═╝  ╚═╝\x1B[37;1m\r
 QUICK START:                         MORE INFO:\x1B[1;30m\r
• WAPM commands:    wapm              • Usage Information: help\r
• Wasmer commands:  wasmer            • About Wasmer: about wasmer\r
• Core utils:       coreutils         • About Deploy: about deploy\r
• Pipe: echo blah | cowsay\r
• Mount: mount --help\x1B[37;1m\r\r\n"#;

    pub const WELCOME_MEDIUM: &'static str = r#"\x1B[1;34m██╗    ██╗ █████╗ ███████╗███╗   ███╗███████╗██████╗ \r
██║    ██║██╔══██╗██╔════╝████╗ ████║██╔════╝██╔══██╗\r
██║ █╗ ██║███████║███████╗██╔████╔██║█████╗  ██████╔╝\r
██║███╗██║██╔══██║╚════██║██║╚██╔╝██║██╔══╝  ██╔══██╗\r
╚███╔███╔╝██║  ██║███████║██║ ╚═╝ ██║███████╗██║  ██║\r
 ╚══╝╚══╝ ╚═╝  ╚═╝╚══════╝╚═╝     ╚═╝╚══════╝╚═╝  ╚═╝\x1B[37;1m\r
 Type 'help' for commands.\x1B[37;1m\r\r\n"#;

    pub const WELCOME_SMALL: &'static str = r#"\x1B[1;34m _ _ _ _____  ___ ____  _____  ____ \r
| | | (____ |/___|    \| ___ |/ ___)\r
| | | / ___ |___ | | | | ____| |    \r
 \___/\_____(___/|_|_|_|_____|_|    \x1B[37;1m\r\r\n"#;

    pub const MOUNT_USAGE: &'static str = r#"Usage:
 mount [<wapm-name>] <mountpoint> <target>

 <wapm-name>: Name of the WAPM program that will serve the file-system (default: tok)
 <mounpoint>: Location where the file-system will be mounted to
 <target>: Target name passed to the WAPM program and is ued for the mounting

 Example: mount tok /www wasmer.sh/wasm
"#;

    pub const UMOUNT_USAGE: &'static str = r#"Usage:
umount <mountpoint>

<mounpoint>: Location where the file-system to be unmounted is currently mounted

Example: umount /www
"#;

    pub const CALL_USAGE: &'static str = r#"Usage:
call <instance> <wapm-name> <topic> [<access-token>]

<instance>: Identifier of the instance that will be invoked
<wapm-name>: Name of the process that will handle this call
<topic>: Target operation that will be invoked
<access-token>: Token used to gain access to this particular instance
<stdin>: Data to be sent to the call (i.e. some json, yaml or binary)
<stdout>: Data returned by the call (i.e. some json, yaml or binary)
"#;

    pub const ABOUT: &'static str = include_str!("txt/about.md");
    pub const ABOUT_DEPLOY: &'static str = include_str!("txt/about_deploy.md");
    pub const ABOUT_WASMER: &'static str = include_str!("txt/about_wasmer.md");
    pub const HELP: &'static str = include_str!("txt/help.md");
    pub const BUILTIN: &'static str = include_str!("txt/builtin.md");
    pub const BAD_WORKER: &'static str = include_str!("txt/bad_worker.md");
}
