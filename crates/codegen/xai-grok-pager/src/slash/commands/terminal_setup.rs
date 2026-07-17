//! `/terminal-setup` — diagnose terminal, color/theme, and clipboard setup.
//!
//! Runs the same diagnostics engine used for startup warnings and formats
//! the results as a user-readable message. This gives users an on-demand
//! way to check their environment and see fix instructions.

use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand};
use crate::terminal::TerminalName;

pub struct TerminalSetupCommand;

impl SlashCommand for TerminalSetupCommand {
    fn name(&self) -> &str {
        "terminal-setup"
    }

    fn aliases(&self) -> &[&str] {
        &["terminal-check", "terminal-info"]
    }

    fn description(&self) -> &str {
        "检查终端、颜色与剪贴板设置"
    }

    fn usage(&self) -> &str {
        "/terminal-setup"
    }

    fn run(&self, _ctx: &mut CommandExecCtx, _args: &str) -> CommandResult {
        let ctx = crate::terminal::terminal_context();
        let query = crate::diagnostics::LiveTmuxQuery;
        let is_control_mode = crate::terminal::detect_tmux_control_mode(ctx);
        let mut warnings = crate::diagnostics::collect_startup_warnings(
            ctx,
            &query,
            is_control_mode,
            _ctx.screen_mode.is_fullscreen(),
        );
        // Live-environment check, kept out of `collect_startup_warnings` so
        // its tests stay hermetic (same pattern as the WezTerm warning below).
        warnings.extend(crate::diagnostics::diagnose_wayland_data_control_live());
        // WezTerm without the Kitty keyboard protocol: surface the fix
        // alongside the other issues. By the time the user runs
        // /terminal-setup the async XTVERSION reply has landed, so this
        // also catches WezTerm over SSH (env brand Unknown, self-report
        // "WezTerm <version>").
        let wezterm_warning = crate::diagnostics::wezterm_kitty_keyboard_warning(
            ctx,
            crate::app::kitty_flags_pushed(),
            crate::terminal::xtversion::detected(),
        );
        let wezterm_kkp_off = wezterm_warning.is_some();
        warnings.extend(wezterm_warning);
        // Color not in collect_startup_warnings (noisy on limited terminals).
        let color_level = crate::theme::color_support::get();
        warnings.extend(crate::diagnostics::color_support_warning(
            color_level,
            ctx.brand,
            ctx.is_tmux_backed(),
            &ctx.tmux_config_path(),
        ));
        let route = crate::clipboard::clipboard_route();
        let is_ssh = xai_grok_shell::util::clipboard::is_remote_session();

        let mut out = String::new();

        // -- Environment --
        out.push_str("环境\n");
        out.push_str(&format!("  终端         {}\n", ctx.brand));
        if let Some(v) = crate::terminal::xtversion::detected() {
            out.push_str(&format!("  终端版本     {}\n", v));
        }
        out.push_str(&format!("  多路复用     {}\n", ctx.multiplexer));
        if let Some(ref byobu) = ctx.byobu {
            out.push_str(&format!("  Byobu        {}\n", byobu));
        }
        out.push_str(&format!(
            "  ssh          {}\n",
            if is_ssh { "是" } else { "否" }
        ));
        out.push_str(&crate::diagnostics::format_color_env_line(color_level));
        out.push_str(&crate::diagnostics::format_themes_env_line(color_level));

        let kb = ctx.keyboard_capabilities();
        if kb.modifier_delivery.benefits_from_rescue() || kb.enter_needs_rescue() {
            let rescue = if cfg!(target_os = "macos") {
                "系统补救已启用"
            } else {
                "本平台无系统补救"
            };
            out.push_str(&format!(
                "  键盘         {} ({})\n",
                kb.modifier_delivery.label(),
                rescue
            ));
        }

        // Some terminals can't distinguish Shift+Enter from bare Enter at
        // the byte level because the Kitty keyboard protocol isn't
        // negotiated (VTE < 0.82, or VS Code's xterm.js which mis-encodes
        // shifted keys). Point users at Alt+Enter, which is reliably
        // delivered as ESC+CR. Suppressed when the WezTerm warning fired:
        // stock WezTerm binds Alt+Enter to ToggleFullScreen, so advertising
        // it would contradict that warning's `\`+Enter guidance.
        if ctx.shift_enter_unavailable() && !wezterm_kkp_off {
            let detail = if ctx.vte_version.is_some() || ctx.brand == TerminalName::Vte {
                match ctx.vte_version.as_deref() {
                    Some(v) => format!("VTE {v}；Shift+Enter 需要 >= 8200"),
                    None => "旧版 VTE；Shift+Enter 需要 VTE >= 0.82".to_owned(),
                }
            } else if matches!(
                ctx.brand,
                TerminalName::VsCode
                    | TerminalName::Cursor
                    | TerminalName::Windsurf
                    | TerminalName::Zed
            ) {
                format!("{}：xterm.js 无法区分 Shift+Enter", ctx.brand)
            } else {
                "无 Kitty 键盘协议；Shift+Enter 等同 Enter".to_owned()
            };
            out.push_str(&format!("  换行         Alt+Enter（{detail}）\n"));
        }

        // -- Clipboard --
        out.push_str("\n剪贴板路径\n");
        out.push_str(&format!(
            "  native       {}  (工具: {})\n",
            if route.native { "启用" } else { "关闭" },
            xai_grok_shell::util::clipboard::native_tool_name(),
        ));
        out.push_str(&format!(
            "  tmux buffer  {}\n",
            if route.tmux_buffer { "启用" } else { "关闭" }
        ));
        out.push_str(&format!(
            "  osc 52       {}\n",
            if route.osc52 { "启用" } else { "关闭" }
        ));
        out.push_str(&format!(
            "  data-control {}\n",
            crate::clipboard::wayland_data_control_label()
        ));

        // -- Diagnostics --
        if warnings.is_empty() {
            out.push_str("\n未发现问题。\n");
        } else {
            out.push_str(&format!("\n{} 个问题\n", warnings.len()));
            for w in &warnings {
                out.push_str(&format!("\n  [!] {}\n", w.message));
                match (w.fix.as_deref(), w.config_path.as_deref()) {
                    (Some(fix), Some(path)) => {
                        out.push_str(&format!("      修复：将 `{}` 写入 {}\n", fix, path));
                    }
                    (Some(fix), None) => {
                        out.push_str(&format!("      修复：运行 `{}`\n", fix));
                    }
                    _ => {}
                }
                if let Some(note) = w.note.as_deref() {
                    out.push_str(&format!("      说明：{}\n", note));
                }
            }
        }

        CommandResult::Message(out)
    }
}
