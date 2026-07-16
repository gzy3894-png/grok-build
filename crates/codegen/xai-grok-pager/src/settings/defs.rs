//! Default settings catalog — declares every user-tunable preference
//! registered in the settings modal.
//!
//! Defaults come from `UiConfig::default()` for SHELL/SHARED settings.
//! The `defaults_match_ui_config_default` test enforces this.

use super::registry::{
    DynamicEnumSource, EnumChoice, SettingCategory, SettingKind, SettingMeta, SettingOwner,
};
use crate::appearance::ScrollMode;
use crate::appearance::TextSelection;
use crate::appearance::permission_cursor::DefaultSelectedPermission;

use xai_grok_shell::agent::config::UiConfig;
use xai_grok_tools::implementations::grok_build::ask_user_question;

// ---------------------------------------------------------------------------
// Int bounds for `max_thoughts_width`.
//
// Stored as `u16` in `UiConfig`, exposed as `i64` for registry uniformity.
// 40 = min readable width on 80-col terminal; 500 = max before
// "obviously wrong" territory. `pub(crate)` so the dispatcher's clamp
// and the shell helper's defensive clamp share these bounds.
pub(crate) const MAX_THOUGHTS_WIDTH_MIN: i64 = 40;
pub(crate) const MAX_THOUGHTS_WIDTH_MAX: i64 = 500;

/// Registry key for `max_thoughts_width`. Shared between the registry
/// definition and the live-wrap-preview gate in the int stepper.
pub(crate) const MAX_THOUGHTS_WIDTH_KEY: &str = "max_thoughts_width";

// ---------------------------------------------------------------------------
// Theme choice catalogs.
//
// Canonical names MUST match `ThemeKind::display_name()`.
// Shared by `theme`, `auto_dark_theme`, and `auto_light_theme`;
// auto-* sub-pickers drop "auto" to avoid circular reference.
// Bounded by `MAX_PICKER_CHOICES`.
// ---------------------------------------------------------------------------

/// Full theme catalog including the "auto" meta-variant. Used by `theme` only.
const THEME_CHOICES: &[EnumChoice] = &[
    EnumChoice {
        canonical: "auto",
        display: "自动",
        description: "跟随系统深色/浅色外观。",
    },
    EnumChoice {
        canonical: "groknight",
        display: "Grok Night",
        description: "中性深色，品红强调色。",
    },
    EnumChoice {
        canonical: "grokday",
        display: "Grok Day",
        description: "明亮环境用的浅色主题。",
    },
    EnumChoice {
        canonical: "tokyonight",
        display: "Tokyo Night",
        description: "深色偏蓝；需要真彩色。",
    },
    // ASCII "Rose Pine Moon" (not "Rosé") for cross-terminal compatibility.
    EnumChoice {
        canonical: "rosepine-moon",
        display: "Rose Pine Moon",
        description: "低饱和深色，淡紫强调；需要真彩色。",
    },
    EnumChoice {
        canonical: "oscura-midnight",
        display: "Oscura Midnight",
        description: "深黑背景，暖色强调；需要真彩色。",
    },
];

// ---------------------------------------------------------------------------
// Permission-mode catalog.
//
// Persisted values map onto runtime flags:
//   "always-approve" ↔ yolo_mode = true  (auto-approve all)
//   "auto"           ↔ auto_mode = true  (LLM classifier; not full yolo)
//   "ask"            ↔ both false (interactive prompts)
//   "default"        ↔ both false (agent's default — currently Ask)
//
// Canonical strings match `load_permission_mode`. `supports_preview:
// false` because toggling YOLO drains the permission queue (unsafe
// for per-keystroke preview).
//
// Adding new modes requires: (1) `PermissionModeKind` variant,
// (2) `EnumChoice` here, (3) `set_yolo_mode_inner` update,
// (4) `load_permission_mode` arm, (5) tests. `Plan` is excluded —
// it lives on its own `plan_mode` setting.
// ---------------------------------------------------------------------------

// Choice order: safe → classifier → unsafe (Default → Ask → Auto → Always approve).
// "Always approve" at the end creates a speed bump against
// accidental selection.
const PERMISSION_MODE_CHOICES: &[EnumChoice] = &[
    // "default" = agent's default behavior. Same as "ask" at runtime;
    // distinct on disk and in the modal indicator.
    EnumChoice {
        canonical: "default",
        display: "默认",
        description: "使用代理默认权限行为（当前等同于「询问」）。",
    },
    EnumChoice {
        canonical: "ask",
        display: "询问",
        description: "执行工具操作前询问权限。",
    },
    EnumChoice {
        canonical: "auto",
        display: "自动",
        description: "由 LLM 分类器批准安全工具；危险操作仍可能询问或拒绝。",
    },
    EnumChoice {
        canonical: "always-approve",
        display: "始终批准",
        description: "自动批准所有工具操作。跳过全部权限确认。",
    },
];

// ---------------------------------------------------------------------------
// Coding-data-sharing catalog.
//
// Persisted in auth metadata (`AuthEntry::coding_data_retention_opt_out`),
// NOT config.toml. Two choices only — the pager has no `Option`/`Unset`
// representation for this field.
//
// `supports_preview: false` — toggling fires an async ACP call that
// can fail. Commit on Enter only.
// ---------------------------------------------------------------------------

const CODING_DATA_SHARING_CHOICES: &[EnumChoice] = &[
    EnumChoice {
        canonical: "opt-in",
        display: "选择加入",
        description: "允许 SpaceXAI 保留并使用编程会话数据以改进训练与产品。",
    },
    EnumChoice {
        canonical: "opt-out",
        display: "选择退出",
        description: "不保留编程会话数据。代码请求不会用于训练。",
    },
];

// ---------------------------------------------------------------------------
// Plan-mode catalog.
//
// PAGER-owned, per-session, ACP-mediated via `session/set_mode`.
// NOT persisted to config.toml — resets every session start.
//
// Uses `on`/`off` canonical strings (not the shell's `plan`/`default`
// wire ids). `Ask` mode is intentionally not exposed here — it's
// only reachable via Shift+Tab.
//
// `supports_preview: false` — toggling fires an ACP request that
// gates tool dispatch. Commit on Enter only.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Default-selected-permission catalog.
//
// Persisted to `[ui].default_selected_permission` in config.toml. Controls
// which row the cursor preselects on the FIRST permission prompt of a
// session; after the user confirms any prompt, the cursor sticks to the
// last-used option kind. `always_allow_all_sessions` (the effective default)
// lands the cursor on the "Always allow on all sessions" / enable-always-approve
// row explicitly, via `is_enable_always_approve_option` — not via index 0; the
// other three map onto `acp::PermissionOptionKind::{AllowOnce, AllowAlways,
// Reject*}`.
//
// `supports_preview: false` — permission prompts aren't open in the modal
// background, so there's no live preview surface.
// ---------------------------------------------------------------------------

// Order matches the live permission prompt rendering (YOLO -> always-allow
// -> allow-once -> reject) so the picker mirrors what the user sees on the
// real prompt.
// Canonicals + display labels come from `DefaultSelectedPermission` (the
// single source of truth) so this table can never drift from the parser,
// the dispatch toast, or the cursor logic.
const DEFAULT_SELECTED_PERMISSION_CHOICES: &[EnumChoice] = &[
    EnumChoice {
        canonical: DefaultSelectedPermission::AlwaysAllowAllSessions.as_canonical(),
        display: DefaultSelectedPermission::AlwaysAllowAllSessions.display(),
        description: "",
    },
    EnumChoice {
        canonical: DefaultSelectedPermission::AllowCommandAlways.as_canonical(),
        display: DefaultSelectedPermission::AllowCommandAlways.display(),
        description: "",
    },
    EnumChoice {
        canonical: DefaultSelectedPermission::AllowOnce.as_canonical(),
        display: DefaultSelectedPermission::AllowOnce.display(),
        description: "",
    },
    EnumChoice {
        canonical: DefaultSelectedPermission::Reject.as_canonical(),
        display: DefaultSelectedPermission::Reject.display(),
        description: "",
    },
];

const PLAN_MODE_CHOICES: &[EnumChoice] = &[
    EnumChoice {
        canonical: "off",
        display: "关",
        description: "代理直接运行工具并编辑文件（默认）。",
    },
    EnumChoice {
        canonical: "on",
        display: "开",
        description: "代理先总结计划，再请求批准后运行工具。",
    },
];

// ---------------------------------------------------------------------------
// Mermaid-rendering catalog.
//
// SHELL-owned: persisted to `[ui].render_mermaid`, with a pager-side
// process-wide cache mirror (`appearance::cache::*_render_mermaid`) for the
// render hot path. Canonicals match `RenderMermaid::as_canonical`.
// ---------------------------------------------------------------------------

const RENDER_MERMAID_CHOICES: &[EnumChoice] = &[
    EnumChoice {
        canonical: "auto",
        display: "自动",
        description: "显示图表，并提供可点击行以打开/复制渲染图。",
    },
    EnumChoice {
        canonical: "on",
        display: "开",
        description: "与自动相同：始终显示可点击操作行。",
    },
    EnumChoice {
        canonical: "off",
        display: "关",
        description: "始终以代码块显示原始 Mermaid 源码。",
    },
];

// Scroll-input catalog. SHELL-owned, persisted to `[ui].scroll_mode`.
// Canonical strings match `ScrollMode::as_canonical` (pinned by test).
const SCROLL_MODE_CHOICES: &[EnumChoice] = &[
    EnumChoice {
        canonical: ScrollMode::Auto.as_canonical(),
        display: "自动检测",
        description: "按手势时序检测滚轮或触控板。默认。",
    },
    EnumChoice {
        canonical: ScrollMode::Wheel.as_canonical(),
        display: "鼠标滚轮",
        description: "始终按滚轮刻度滚动（每跳固定行数）。",
    },
    EnumChoice {
        canonical: ScrollMode::Trackpad.as_canonical(),
        display: "触控板",
        description: "始终按触控板滚动（小数累积）。",
    },
];

const TEXT_SELECTION_CHOICES: &[EnumChoice] = &[
    EnumChoice {
        canonical: TextSelection::Flash.as_canonical(),
        display: "复制后闪一下",
        description: "松开鼠标后短暂高亮再清除。双击切换折叠。默认。",
    },
    EnumChoice {
        canonical: TextSelection::Hold.as_canonical(),
        display: "保持到关闭",
        description: "选择保持可见，直到 Esc、点击或滚动。双击切换折叠。",
    },
    EnumChoice {
        canonical: TextSelection::WordSelect.as_canonical(),
        display: "选词（终端风格）",
        description: "双击选中并复制一词，三击选中一行；选择保持到关闭。",
    },
];

// Hunk-tracker-mode catalog. SHELL-owned, persisted to `[ui].hunk_tracker_mode`.
// `disabled` is accepted as an alias for `off` at parse time but not surfaced
// as a choice.
const HUNK_TRACKER_MODE_CHOICES: &[EnumChoice] = &[
    EnumChoice {
        canonical: "agent_only",
        display: "仅代理",
        description: "仅跟踪代理编辑的文件（默认）。",
    },
    EnumChoice {
        canonical: "all_dirty",
        display: "全部脏文件",
        description: "跟踪全部 git 脏文件，含外部编辑。",
    },
    EnumChoice {
        canonical: "off",
        display: "关",
        description: "完全禁用 hunk 跟踪，同时关闭 LOC 跟踪。",
    },
];

const SCREEN_MODE_CHOICES: &[EnumChoice] = &[
    EnumChoice {
        canonical: "fullscreen",
        display: "全屏",
        description: "以标准全屏 TUI 打开 grok。未设置时的默认。",
    },
    EnumChoice {
        canonical: "minimal",
        display: "极简",
        description: "以回滚原生（极简）模式打开 grok。",
    },
];

// Voice-capture-mode catalog. SHELL-owned, persisted to `[ui].voice_capture_mode`.
// `hold` is only offered on terminals that report key releases (Kitty keyboard
// protocol); `effective_enum_choices` hides it elsewhere, and it falls back to
// `toggle` at runtime.
const VOICE_CAPTURE_MODE_CHOICES: &[EnumChoice] = &[
    EnumChoice {
        canonical: "toggle",
        display: "切换",
        description: "Ctrl+Space / F8 开始听写；再按一次（或 Esc/Enter）停止。",
    },
    EnumChoice {
        canonical: "hold",
        display: "按住说话",
        description: "按住 Ctrl+Space / F8 录音，松开停止。需要支持 Kitty 协议的终端。",
    },
];

// Voice STT language choices for the settings modal.
//
// Concrete codes must match `xai_grok_voice::STT_LANGUAGES` (official Grok STT
// catalog — https://docs.x.ai/developers/model-capabilities/audio/speech-to-text).
// `auto` is client-only; the voice crate resolves it to a concrete code before
// the STT handshake. Order: English (default), System, then remaining languages
// A–Z by English name. A registry unit test locks this list to the voice crate.
const VOICE_STT_LANGUAGE_CHOICES: &[EnumChoice] = &[
    EnumChoice {
        canonical: "en",
        display: "English",
        description: "",
    },
    EnumChoice {
        canonical: "auto",
        display: "系统",
        description: "系统区域设置若为支持的听写语言则使用，否则英语。",
    },
    EnumChoice {
        canonical: "ar",
        display: "Arabic",
        description: "",
    },
    EnumChoice {
        canonical: "cs",
        display: "Czech",
        description: "",
    },
    EnumChoice {
        canonical: "da",
        display: "Danish",
        description: "",
    },
    EnumChoice {
        canonical: "nl",
        display: "Dutch",
        description: "",
    },
    EnumChoice {
        canonical: "fil",
        display: "Filipino",
        description: "",
    },
    EnumChoice {
        canonical: "fr",
        display: "French",
        description: "",
    },
    EnumChoice {
        canonical: "de",
        display: "German",
        description: "",
    },
    EnumChoice {
        canonical: "hi",
        display: "Hindi",
        description: "",
    },
    EnumChoice {
        canonical: "id",
        display: "Indonesian",
        description: "",
    },
    EnumChoice {
        canonical: "it",
        display: "Italian",
        description: "",
    },
    EnumChoice {
        canonical: "ja",
        display: "Japanese",
        description: "",
    },
    EnumChoice {
        canonical: "ko",
        display: "Korean",
        description: "",
    },
    EnumChoice {
        canonical: "mk",
        display: "Macedonian",
        description: "",
    },
    EnumChoice {
        canonical: "ms",
        display: "Malay",
        description: "",
    },
    EnumChoice {
        canonical: "fa",
        display: "Persian",
        description: "",
    },
    EnumChoice {
        canonical: "pl",
        display: "Polish",
        description: "",
    },
    EnumChoice {
        canonical: "pt",
        display: "Portuguese",
        description: "",
    },
    EnumChoice {
        canonical: "ro",
        display: "Romanian",
        description: "",
    },
    EnumChoice {
        canonical: "ru",
        display: "Russian",
        description: "",
    },
    EnumChoice {
        canonical: "es",
        display: "Spanish",
        description: "",
    },
    EnumChoice {
        canonical: "sv",
        display: "Swedish",
        description: "",
    },
    EnumChoice {
        canonical: "th",
        display: "Thai",
        description: "",
    },
    EnumChoice {
        canonical: "tr",
        display: "Turkish",
        description: "",
    },
    EnumChoice {
        canonical: "vi",
        display: "Vietnamese",
        description: "",
    },
];

/// Concrete-only theme catalog (excludes "auto"). Used by both
/// `auto_dark_theme` and `auto_light_theme`. No dark/light filtering —
/// the user can pair any theme with any system-appearance bucket.
const CONCRETE_THEME_CHOICES: &[EnumChoice] = &[
    EnumChoice {
        canonical: "groknight",
        display: "Grok Night",
        description: "中性深色，品红强调色。",
    },
    EnumChoice {
        canonical: "grokday",
        display: "Grok Day",
        description: "明亮环境用的浅色主题。",
    },
    EnumChoice {
        canonical: "tokyonight",
        display: "Tokyo Night",
        description: "深色偏蓝；需要真彩色。",
    },
    EnumChoice {
        canonical: "rosepine-moon",
        display: "Rose Pine Moon",
        description: "低饱和深色，淡紫强调；需要真彩色。",
    },
    EnumChoice {
        canonical: "oscura-midnight",
        display: "Oscura Midnight",
        description: "深黑背景，暖色强调；需要真彩色。",
    },
];

/// Child settings shown inside the "Show contextual hints" group sub-sheet.
/// Keys match the `[ui.contextual_hints]` serde fields (namespaced so they stay
/// globally unique — bare `plan_mode` collides with the plan-mode enum row).
/// They are registered as normal Bool settings but hidden from the top-level
/// list (`build_rows` skips any key that is a group child).
const CONTEXTUAL_HINTS_CHILDREN: &[&str] = &[
    "contextual_hints.undo",
    "contextual_hints.plan_mode",
    "contextual_hints.image_input",
    "contextual_hints.send_now",
    "contextual_hints.small_screen",
    "contextual_hints.word_select",
];

/// Build the catalog. Called once at process start via
/// `SettingsRegistry::defaults()`.
pub fn default_settings() -> Vec<SettingMeta> {
    // Shell schema defaults, used as registry source of truth.
    let ui_default = UiConfig::default();

    vec![
        SettingMeta {
            key: "compact_mode",
            category: SettingCategory::Appearance,
            owner: SettingOwner::Shared,
            label: "紧凑模式",
            description: "减少消息周围留白以提高内容密度。\
                          终端高度 ≤20 行时自动开启。",
            keywords: &[
                "compact", "density", "padding", "tight", "small", "screen", "auto",
            ],
            kind: SettingKind::Bool {
                default: ui_default.compact_mode,
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        SettingMeta {
            key: "screen_mode",
            category: SettingCategory::Appearance,
            owner: SettingOwner::Shell,
            label: "默认屏幕模式",
            description: "下次启动 plain grok 的方式：全屏（未设置时默认）或\
                          极简。写入 config.toml 的 [ui] screen_mode。需重启。\
                          仅本会话可用 /minimal 或 /fullscreen 切换。",
            keywords: &[
                "screen",
                "mode",
                "minimal",
                "fullscreen",
                "full",
                "scrollback",
                "native",
                "alt-screen",
                "render",
                "default",
            ],
            kind: SettingKind::Enum {
                default: "fullscreen",
                choices: SCREEN_MODE_CHOICES,
                supports_preview: false,
            },
            restart_required: true,
            hidden_in_minimal: false,
        },
        SettingMeta {
            key: "show_timestamps",
            category: SettingCategory::Appearance,
            owner: SettingOwner::Shared,
            label: "显示时间戳",
            description: "在用户消息与代理回复旁显示时钟时间。",
            keywords: &["timestamps", "time", "clock", "date"],
            kind: SettingKind::Bool {
                // `Option<bool>` — `None` treated as `true`.
                default: ui_default.show_timestamps.unwrap_or(true),
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        SettingMeta {
            key: "show_timeline",
            category: SettingCategory::Appearance,
            owner: SettingOwner::Shared,
            label: "时间线侧栏",
            description: "用每回合刻度条替代滚动条：悬停预览回合，点击跳转。",
            keywords: &["timeline", "sidebar", "ticks", "turns", "navigator", "rail"],
            kind: SettingKind::Bool {
                // Single source: UiConfig::SHOW_TIMELINE_DEFAULT (opt-in).
                default: ui_default.show_timeline_enabled(),
            },
            restart_required: false,
            // Minimal mode has no interactive scrollback pane for the rail.
            hidden_in_minimal: true,
        },
        SettingMeta {
            // Persisted key stays `simple_mode`; the user-facing label
            // distinguishes the PROMPT vim-mode (this setting) from the
            // scrollback `vim_mode` keybindings below.
            key: "simple_mode",
            category: SettingCategory::Appearance,
            owner: SettingOwner::Shared,
            label: "禁用 Vim 输入模式",
            description: "提示框使用普通 readline 风格输入，而非 vim 键位。实验性。",
            keywords: &[
                "simple",
                "ascii",
                "minimal",
                "plain",
                "vim",
                "readline",
                "experimental",
                "editor",
                "input",
                "prompt",
            ],
            kind: SettingKind::Bool {
                // `Option<bool>` — `None` treated as `true`.
                default: ui_default.simple_mode.unwrap_or(true),
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        // SHELL-owned, persisted to `[ui].vim_mode` in config.toml.
        // Defaults to the same value main's `appearance::persist::VIM_MODE_DEFAULT`
        // shipped with. Bundled next to `simple_mode` because they pair up:
        // simple_mode controls the input editor's vim behaviour,
        // vim_mode controls the scrollback's vim behaviour.
        SettingMeta {
            key: "vim_mode",
            category: SettingCategory::Appearance,
            owner: SettingOwner::Shell,
            label: "Vim 回滚导航",
            description: "用 vim 键（h/j/k/l、gg/G、/）导航回滚。不影响输入提示框。",
            keywords: &[
                "vim",
                "scrollback",
                "navigation",
                "hjkl",
                "keys",
                "keybindings",
                "scroll",
            ],
            kind: SettingKind::Bool {
                default: ui_default.vim_mode.unwrap_or(false),
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        // --- theme + auto themes ---------------------------------------------
        SettingMeta {
            key: "theme",
            category: SettingCategory::Appearance,
            owner: SettingOwner::Shared,
            label: "主题",
            description: "分页器界面的配色主题。",
            keywords: &[
                "theme",
                "color",
                "colour",
                "palette",
                "appearance",
                "dark",
                "light",
            ],
            kind: SettingKind::Enum {
                // `Option<String>` — `None` resolved to "groknight".
                default: "groknight",
                choices: THEME_CHOICES,
                supports_preview: true,
            },
            restart_required: false,
            hidden_in_minimal: true,
        },
        SettingMeta {
            key: "auto_dark_theme",
            category: SettingCategory::Appearance,
            owner: SettingOwner::Shared,
            label: "自动深色主题",
            description: "系统为深色模式时使用的主题（仅 theme=auto 时）。",
            keywords: &["auto", "dark", "theme", "system", "appearance", "night"],
            kind: SettingKind::Enum {
                // `Option<String>` — `None` falls back to "groknight".
                default: "groknight",
                choices: CONCRETE_THEME_CHOICES,
                supports_preview: true,
            },
            restart_required: false,
            hidden_in_minimal: true,
        },
        SettingMeta {
            key: "auto_light_theme",
            category: SettingCategory::Appearance,
            owner: SettingOwner::Shared,
            label: "自动浅色主题",
            description: "系统为浅色模式时使用的主题（仅 theme=auto 时）。",
            keywords: &["auto", "light", "theme", "system", "appearance", "day"],
            kind: SettingKind::Enum {
                // `Option<String>` — `None` falls back to "grokday".
                default: "grokday",
                choices: CONCRETE_THEME_CHOICES,
                supports_preview: true,
            },
            restart_required: false,
            hidden_in_minimal: true,
        },
        // SHELL-owned: persisted to `[ui].render_mermaid`, with a pager-side
        // process-wide cache mirror (like `vim_mode`). Default pinned to "auto"
        // by `defaults_match_ui_config_default`.
        SettingMeta {
            key: "render_mermaid",
            category: SettingCategory::Appearance,
            owner: SettingOwner::Shell,
            label: "渲染 Mermaid 图",
            description: "```mermaid 代码块的显示方式：auto/on 增加可点击行以\
                          打开渲染图；off 显示原始源码。",
            keywords: &[
                "mermaid",
                "diagram",
                "diagrams",
                "render",
                "flowchart",
                "graph",
                "chart",
            ],
            kind: SettingKind::Enum {
                default: "auto",
                choices: RENDER_MERMAID_CHOICES,
                supports_preview: false,
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        // Security-relevant: "always-approve" bypasses all permission prompts.
        // Modal reads live state from `PagerLocalSnapshot.yolo_mode`
        // (not `ui.permission_mode`) to reflect Ctrl+O toggles immediately.
        SettingMeta {
            key: "permission_mode",
            category: SettingCategory::Agent,
            owner: SettingOwner::Shell,
            label: "权限模式",
            description: "默认使用代理内置行为；\
                          「询问」在每次工具操作前确认；\
                          「自动」用 LLM 分类器处理风险工具；\
                          「始终批准」自动授予全部权限。",
            keywords: &[
                "permission",
                "approve",
                "yolo",
                "agent",
                "always",
                "ask",
                "auto",
                "classifier",
                "tool",
                "danger",
            ],
            kind: SettingKind::Enum {
                default: "ask",
                choices: PERMISSION_MODE_CHOICES,
                supports_preview: false,
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        // SHELL-owned `[ui].remember_tool_approvals`. Gates the per-tool
        // "Always allow …" prompt options. `restart_required` — resolved at
        // permission-manager spawn (also fed by env/requirements/managed/remote settings).
        SettingMeta {
            key: "remember_tool_approvals",
            category: SettingCategory::Agent,
            owner: SettingOwner::Shell,
            label: "记住工具批准",
            description: "在权限提示中显示「始终允许」选项，避免对同一命令或工具\
                          反复询问。适用于询问/自动模式；始终批准仍会跳过全部提示。需重启。",
            keywords: &[
                "permission",
                "approve",
                "approval",
                "always",
                "allow",
                "remember",
                "tool",
                "command",
                "kubectl",
                "ask",
                "again",
                "whitelist",
            ],
            kind: SettingKind::Bool {
                default: ui_default.remember_tool_approvals.unwrap_or(false),
            },
            restart_required: true,
            hidden_in_minimal: false,
        },
        // PAGER-owned; default pinned by `defaults_match_pager_state`.
        SettingMeta {
            key: "multiline_mode",
            category: SettingCategory::Editor,
            owner: SettingOwner::Pager,
            label: "多行",
            description: "开启后 Enter 换行，Shift+Enter 发送。每会话重置。",
            keywords: &["multiline", "newline", "input", "editor", "enter"],
            kind: SettingKind::Bool { default: false },
            restart_required: false,
            hidden_in_minimal: false,
        },
        // SHELL-owned. Reads from `pager.current_model_name` (not
        // `cfg.models.default`) so the modal reflects `/model` switches.
        // Empty-string default = "no opinion" / use shell's resolution.
        SettingMeta {
            key: "default_model",
            category: SettingCategory::Models,
            owner: SettingOwner::Shell,
            label: "默认模型",
            description: "新会话使用的模型。更改也会切换当前会话。选「(无覆盖)」可清除。",
            keywords: &["model", "default", "agent", "llm", "grok", "switch"],
            kind: SettingKind::DynamicEnum {
                default: "",
                source: DynamicEnumSource::ActiveModelCatalog,
                supports_preview: false,
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        // SHARED. `u16` in UiConfig, widened to `i64` for registry.
        // Width changes apply on the next render frame.
        SettingMeta {
            key: MAX_THOUGHTS_WIDTH_KEY,
            category: SettingCategory::Appearance,
            owner: SettingOwner::Shared,
            label: "思考块最大宽度",
            description: "代理思考面板的列宽预算（40–500，默认 120）。",
            keywords: &[
                "thoughts",
                "width",
                "max",
                "thinking",
                "panel",
                "reasoning",
                "columns",
            ],
            kind: SettingKind::Int {
                default: ui_default.max_thoughts_width as i64,
                min: MAX_THOUGHTS_WIDTH_MIN,
                max: MAX_THOUGHTS_WIDTH_MAX,
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        // SHELL-owned: `[ui].show_thinking_blocks` + process-wide cache. Default ON.
        SettingMeta {
            key: "show_thinking_blocks",
            category: SettingCategory::Appearance,
            owner: SettingOwner::Shell,
            label: "显示思考块",
            description: "流式输出时在回滚中显示代理思考/推理块。",
            keywords: &[
                "thinking",
                "reasoning",
                "thoughts",
                "blocks",
                "show",
                "hide",
            ],
            kind: SettingKind::Bool {
                default: ui_default.show_thinking_blocks.unwrap_or(true),
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        // SHELL-owned: `[ui].prompt_suggestions` + process-wide cache. Default ON.
        // The `GROK_PROMPT_SUGGESTIONS` env var overrides at runtime.
        SettingMeta {
            key: "prompt_suggestions",
            category: SettingCategory::Editor,
            owner: SettingOwner::Shell,
            label: "提示建议",
            description: "每回合结束后预测你可能的下一条提示，并以\
                          幽灵文字显示在输入框（Tab 接受）。每回合会有一次小模型调用。",
            keywords: &[
                "prompt",
                "suggestion",
                "suggestions",
                "autocomplete",
                "ghost",
                "tab",
                "predict",
                "next",
            ],
            kind: SettingKind::Bool {
                default: ui_default.prompt_suggestions.unwrap_or(true),
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        // PAGER-owned, persisted to `[scrollback.scroll].respect_manual_folds`
        // in pager.toml (NOT config.toml). Live value is the appearance
        // config (`AppView::set_appearance` fans changes out to every agent);
        // the flag is read at use time, so no restart.
        SettingMeta {
            key: "respect_manual_folds",
            category: SettingCategory::Appearance,
            owner: SettingOwner::Pager,
            label: "保留手动折叠",
            description: "流式输出时保留手动折叠块，展开块时停止\
                          自动滚动。实验性。",
            keywords: &[
                "fold", "pin", "collapse", "expand", "thinking", "follow", "scroll",
            ],
            kind: SettingKind::Bool {
                default: crate::appearance::ScrollConfig::default().respect_manual_folds,
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        // SHELL-owned: `[ui].group_tool_verbs` + process-wide cache. Default ON.
        SettingMeta {
            key: "group_tool_verbs",
            category: SettingCategory::Appearance,
            owner: SettingOwner::Shell,
            label: "合并工具调用",
            description: "将连续的 read/search/list 工具调用与子代理行折叠为\
                          一行摘要；已完成的思考也会并入分组。",
            keywords: &[
                "group", "tool", "verbs", "fold", "collapse", "read", "search", "summary",
                "thinking", "subagent",
            ],
            kind: SettingKind::Bool {
                default: ui_default.group_tool_verbs.unwrap_or(true),
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        // SHELL-owned: `[ui].collapsed_edit_blocks` + process-wide cache.
        // Default OFF (rollout flag; remote settings / managed config can enable).
        SettingMeta {
            key: "collapsed_edit_blocks",
            category: SettingCategory::Appearance,
            owner: SettingOwner::Shell,
            label: "折叠编辑块",
            description: "将编辑显示为一行 +N/-M 摘要，并把同一文件连续编辑合并为一块；展开行可查看 diff。",
            keywords: &[
                "edit",
                "edits",
                "diff",
                "diffstat",
                "collapse",
                "collapsed",
                "summary",
                "expand",
                "one-line",
                "merge",
                "coalesce",
            ],
            kind: SettingKind::Bool {
                default: ui_default.collapsed_edit_blocks.unwrap_or(false),
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        // SHELL-owned: `[ui.display_refresh].auto_cadence_enabled`. Restart-
        // required (cadence pinned at startup); hidden in minimal.
        SettingMeta {
            key: "display_refresh_auto_cadence",
            category: SettingCategory::Appearance,
            owner: SettingOwner::Shell,
            label: "匹配显示刷新率",
            description: "在高刷新率显示器上，TUI 会更快地流式输出/滚动\
                          以匹配刷新率。关闭则保持约 60Hz。需重启。",
            keywords: &[
                "display", "refresh", "rate", "hz", "cadence", "fps", "smooth", "scroll", "stream",
                "high", "120", "144",
            ],
            kind: SettingKind::Bool {
                default: ui_default
                    .display_refresh
                    .auto_cadence_enabled
                    .unwrap_or(false),
            },
            restart_required: true,
            hidden_in_minimal: true,
        },
        // SHELL-owned, persisted to `[ui].scroll_speed` in config.toml.
        SettingMeta {
            key: "scroll_speed",
            category: SettingCategory::Mouse,
            owner: SettingOwner::Shell,
            label: "滚动速度",
            description: "鼠标滚轮与触控板滚动速度倍率（1–100）。越大越快。",
            keywords: &[
                "scroll", "speed", "mouse", "wheel", "trackpad", "fast", "slow",
            ],
            kind: SettingKind::Int {
                default: ui_default.scroll_speed.unwrap_or(50) as i64,
                min: 1,
                max: 100,
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        // SHELL-owned `auto` | `wheel` | `trackpad` on `[ui].scroll_mode`.
        SettingMeta {
            key: "scroll_mode",
            category: SettingCategory::Mouse,
            owner: SettingOwner::Shell,
            label: "滚动输入",
            description: "自动检测误判设备时，强制使用滚轮或触控板滚动行为。",
            keywords: &[
                "scroll", "mode", "wheel", "trackpad", "mouse", "detect", "force", "input",
            ],
            kind: SettingKind::Enum {
                default: ui_default
                    .scroll_mode
                    .as_deref()
                    .and_then(ScrollMode::from_canonical)
                    .unwrap_or_default()
                    .as_canonical(),
                choices: SCROLL_MODE_CHOICES,
                supports_preview: false,
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        // SHELL-owned, persisted to `[ui].scroll_lines`. One knob for BOTH
        // wheel and trackpad lines-per-tick; the registered default 3 matches
        // most terminal profiles, but until the user first commits a value
        // the per-terminal profile stays in charge (cache unset → no override).
        SettingMeta {
            key: "scroll_lines",
            category: SettingCategory::Mouse,
            owner: SettingOwner::Shell,
            label: "滚动行数",
            description: "滚轮与触控板每跳滚动行数（1–10）。\
                          未设置前沿用各终端自身配置。",
            keywords: &[
                "scroll", "lines", "tick", "notch", "wheel", "trackpad", "mouse",
            ],
            kind: SettingKind::Int {
                default: ui_default.scroll_lines.map(i64::from).unwrap_or(3),
                min: 1,
                max: 10,
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        // SHELL-owned: `[ui].invert_scroll` + process-wide cache. Default OFF.
        SettingMeta {
            key: "invert_scroll",
            category: SettingCategory::Mouse,
            owner: SettingOwner::Shell,
            label: "反转滚动",
            description: "反转垂直滚动方向（自然滚动）。",
            keywords: &[
                "invert",
                "scroll",
                "natural",
                "direction",
                "reverse",
                "mouse",
                "trackpad",
            ],
            kind: SettingKind::Bool {
                default: ui_default.invert_scroll.unwrap_or(false),
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        // SHELL-owned `flash` | `hold` on `[ui].keep_text_selection`.
        SettingMeta {
            key: "keep_text_selection",
            category: SettingCategory::Mouse,
            owner: SettingOwner::Shell,
            label: "文本选择",
            description: "应用内选择在屏幕上保持多久，以及双击行为（折叠 vs 选词并复制）。终端/多路复用器自身的选择请按住 Shift 拖动（原生复制）。",
            keywords: &[
                "selection",
                "drag",
                "copy",
                "flash",
                "hold",
                "shift",
                "native",
                "mouse",
                "tmux",
                "double",
                "double-click",
                "word",
                "terminal",
            ],
            kind: SettingKind::Enum {
                default: TextSelection::Flash.as_canonical(),
                choices: TEXT_SELECTION_CHOICES,
                supports_preview: false,
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        // SHELL-owned. Persisted in auth metadata (not config.toml).
        // Reads from `PagerLocalSnapshot.coding_data_sharing_opt_out`.
        // Default "opt-in" matches `AuthEntry::coding_data_retention_opt_out = false`.
        // ZDR / non-admin guards are enforced at dispatch time.
        SettingMeta {
            key: "coding_data_sharing",
            category: SettingCategory::Privacy,
            owner: SettingOwner::Shell,
            label: "编程数据共享",
            description: "控制 SpaceXAI 是否可保留并以编程会话数据做训练。",
            keywords: &[
                "privacy",
                "data",
                "sharing",
                "coding",
                "retention",
                "telemetry",
                "training",
                "opt-in",
                "opt-out",
            ],
            kind: SettingKind::Enum {
                default: "opt-in",
                choices: CODING_DATA_SHARING_CHOICES,
                supports_preview: false,
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        // SHELL-owned, persisted to `[ui].default_selected_permission` in
        // config.toml. Read by the pager via `appearance::permission_cursor`.
        // Canonical `always_allow_all_sessions` (the effective default) lands
        // the first prompt's cursor on the enable-always-approve row;
        // subsequent prompts stick to the last-used kind.
        SettingMeta {
            key: "default_selected_permission",
            category: SettingCategory::Agent,
            owner: SettingOwner::Shell,
            label: "默认选中的权限选项",
            description: "权限提示中光标预选中的选项行。",
            keywords: &[
                "permission",
                "approval",
                "cursor",
                "preselect",
                "default",
                "sticky",
                "last",
                "used",
                "yes",
                "no",
                "reject",
                "allow",
            ],
            kind: SettingKind::Enum {
                default: DefaultSelectedPermission::AlwaysAllowAllSessions.as_canonical(),
                choices: DEFAULT_SELECTED_PERMISSION_CHOICES,
                supports_preview: false,
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        // SHELL-owned `[toolset.ask_user_question].timeout_enabled`. Surfaces
        // the user-config layer of the tiered timeout gate (requirements/env/
        // managed/remote settings feed the effective value at agent build); the
        // default is the resolver-shared const. `restart_required` — resolved
        // when an agent is built, like `remember_tool_approvals`.
        SettingMeta {
            key: "toolset.ask_user_question.timeout_enabled",
            category: SettingCategory::Agent,
            owner: SettingOwner::Shell,
            label: "提问超时",
            description: "开启后，ask_user_question 工具会在设定时间后超时，\
                          而不是无限阻塞。",
            keywords: &[
                "ask",
                "question",
                "questionnaire",
                "timeout",
                "ask_user_question",
                "block",
                "wait",
                "forever",
                "tool",
            ],
            kind: SettingKind::Bool {
                default: ask_user_question::DEFAULT_ASK_USER_QUESTION_TIMEOUT_ENABLED,
            },
            restart_required: true,
            hidden_in_minimal: false,
        },
        // PAGER-owned, ACP-mediated. Reads from
        // `PagerLocalSnapshot.plan_mode_active`. Default "off" matches
        // `AgentView::new`'s `plan_mode_active = false`.
        SettingMeta {
            key: "plan_mode",
            category: SettingCategory::Agent,
            owner: SettingOwner::Pager,
            label: "计划模式",
            description: "开启后，代理在运行工具或编辑前会先总结计划。",
            keywords: &[
                "plan", "mode", "agent", "summary", "approval", "review", "session",
            ],
            kind: SettingKind::Enum {
                default: "off",
                choices: PLAN_MODE_CHOICES,
                supports_preview: false,
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        // SHELL-owned startup-time settings (restart_required: true).
        // The running pager doesn't re-read these mid-session.
        SettingMeta {
            key: "show_tips",
            category: SettingCategory::Advanced,
            owner: SettingOwner::Shell,
            label: "显示提示",
            description: "启动时显示每日提示横幅。需重启生效。",
            keywords: &[
                "tips", "tip", "show", "banner", "welcome", "startup", "launch",
            ],
            kind: SettingKind::Bool { default: true },
            restart_required: true,
            hidden_in_minimal: false,
        },
        // Contextual hints: one Advanced row that opens a sub-sheet of per-tip
        // toggles. Applies live (restart_required: false); the group carries no
        // value and its children are hidden from the top-level list.
        SettingMeta {
            key: "contextual_hints",
            category: SettingCategory::Advanced,
            owner: SettingOwner::Shell,
            label: "显示情境提示",
            description: "工作时显示简短的情境快捷键提示；\
                          可逐项开关。",
            keywords: &[
                "contextual",
                "hints",
                "tips",
                "undo",
                "plan",
                "nudge",
                "image",
                "clipboard",
                "ephemeral",
                "send",
                "interject",
                "queue",
                // Child-specific terms: the per-tip children are hidden from the
                // top-level list, so mirror their search words here to keep a
                // query like "ctrl+z" or "shift+tab" from dead-ending.
                "ctrl+z",
                "draft",
                "wipe",
                "mode",
                "shift+tab",
                "paste",
                "input",
                "enter",
                "follow-up",
                "small",
                "screen",
                "compact",
            ],
            kind: SettingKind::Group {
                children: CONTEXTUAL_HINTS_CHILDREN,
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        SettingMeta {
            key: "auto_update",
            category: SettingCategory::Advanced,
            owner: SettingOwner::Shell,
            label: "自动更新",
            description: "启动时自动下载并安装分页器更新。\
                          需重启。",
            keywords: &[
                "auto", "update", "updates", "upgrade", "version", "install", "channel",
            ],
            kind: SettingKind::Bool { default: true },
            restart_required: true,
            hidden_in_minimal: false,
        },
        // SHELL-owned, persisted to `[ui].hunk_tracker_mode`. Restart-required:
        // the mode is read once when the session connects.
        SettingMeta {
            key: "hunk_tracker_mode",
            category: SettingCategory::Advanced,
            owner: SettingOwner::Shell,
            label: "改动块跟踪",
            description: "代理以 hunk 跟踪哪些文件改动。\
                          关则完全禁用跟踪（及 LOC 统计）。\
                          需重启。",
            keywords: &[
                "hunk", "tracker", "tracking", "diff", "changes", "git", "loc", "off", "disable",
            ],
            kind: SettingKind::Enum {
                default: "agent_only",
                choices: HUNK_TRACKER_MODE_CHOICES,
                supports_preview: false,
            },
            restart_required: true,
            hidden_in_minimal: false,
        },
        // SHELL-owned, persisted to `[ui].voice_capture_mode`. The `hold` choice
        // is hidden on terminals without key-release reporting (see
        // `effective_enum_choices`) and falls back to `toggle` at runtime.
        SettingMeta {
            key: "voice_capture_mode",
            category: SettingCategory::Editor,
            owner: SettingOwner::Shell,
            label: "语音采集",
            description: "语音快捷键（Ctrl+Space / F8）行为：切换\
                          （按一下开始/停止）或按住说话（按住录音，\
                          松开停止；需要 Kitty 协议终端）。",
            keywords: &[
                "voice",
                "dictation",
                "dictate",
                "mic",
                "microphone",
                "speech",
                "stt",
                "toggle",
                "hold",
                "ctrl+space",
                "f8",
                "push-to-talk",
            ],
            kind: SettingKind::Enum {
                default: "hold",
                choices: VOICE_CAPTURE_MODE_CHOICES,
                supports_preview: false,
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        // SHELL-owned, persisted to `[ui].voice_stt_language`. Live-applied to
        // the next voice capture (no restart). Default English; System (`auto`)
        // follows the process locale when it maps to a Grok STT language.
        // Catalog = official STT languages (see xai_grok_voice::STT_LANGUAGES).
        SettingMeta {
            key: "voice_stt_language",
            category: SettingCategory::Editor,
            owner: SettingOwner::Shell,
            label: "语音语言",
            description: "语音听写的语音识别语言（Grok STT）。\
                          默认英语；「系统」在支持时使用区域设置。\
                          同时决定数字与货币的格式语言。",
            keywords: &["voice", "language", "locale", "dictation", "stt", "speech"],
            kind: SettingKind::Enum {
                default: "en",
                choices: VOICE_STT_LANGUAGE_CHOICES,
                supports_preview: false,
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        // Contextual-hint children (hidden from the top-level list; reached via
        // the group sub-sheet). Default ON — `None` (inherit) reads as `true`.
        SettingMeta {
            key: "contextual_hints.undo",
            category: SettingCategory::Advanced,
            owner: SettingOwner::Shell,
            label: "撤销",
            description: "清空提示后提醒可用 Ctrl+Z 恢复。",
            keywords: &["undo", "ctrl+z", "draft", "wipe", "hint"],
            kind: SettingKind::Bool {
                default: ui_default.contextual_hints.undo.unwrap_or(true),
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        SettingMeta {
            key: "contextual_hints.plan_mode",
            category: SettingCategory::Advanced,
            owner: SettingOwner::Shell,
            label: "计划模式",
            description: "当提示像是规划请求时，建议使用计划模式（Shift+Tab）。",
            keywords: &["plan", "mode", "nudge", "shift+tab", "hint"],
            kind: SettingKind::Bool {
                default: ui_default.contextual_hints.plan_mode.unwrap_or(true),
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        SettingMeta {
            key: "contextual_hints.image_input",
            category: SettingCategory::Advanced,
            owner: SettingOwner::Shell,
            label: "图片输入",
            description: "剪贴板有图片且模型支持图片时，提示可粘贴。",
            keywords: &["image", "clipboard", "paste", "input", "hint"],
            kind: SettingKind::Bool {
                default: ui_default.contextual_hints.image_input.unwrap_or(true),
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        SettingMeta {
            key: "contextual_hints.send_now",
            category: SettingCategory::Advanced,
            owner: SettingOwner::Shell,
            label: "立即发送",
            description: "回合中途排队后续消息后，提醒空提示框按 Enter\
                          可立即发送队首项。",
            keywords: &[
                "send",
                "now",
                "interject",
                "queue",
                "follow-up",
                "enter",
                "empty",
                "hint",
            ],
            kind: SettingKind::Bool {
                default: ui_default.contextual_hints.send_now.unwrap_or(true),
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        SettingMeta {
            key: "contextual_hints.small_screen",
            category: SettingCategory::Advanced,
            owner: SettingOwner::Shell,
            label: "小屏",
            description: "终端行数较少时，每次运行建议一次 /compact-mode。",
            keywords: &["small", "screen", "compact", "space", "rows", "hint"],
            kind: SettingKind::Bool {
                default: ui_default.contextual_hints.small_screen.unwrap_or(true),
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        SettingMeta {
            key: "contextual_hints.word_select",
            category: SettingCategory::Advanced,
            owner: SettingOwner::Shell,
            label: "词选择",
            description: "在文本选择为折叠/导航时双击对话文字后，\
                          提醒「选词」在设置中。",
            keywords: &[
                "word",
                "select",
                "double",
                "double-click",
                "click",
                "fold",
                "selection",
                "settings",
                "hint",
            ],
            kind: SettingKind::Bool {
                default: ui_default.contextual_hints.word_select.unwrap_or(true),
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
        // ── TodoGate (runtime turn-end backstop) ──────────────────────
        //
        // Only the CLI flag (`--todo-gate`) is wired. Settings-modal
        // entries for `[reminder.todo_gate]` are deferred — the modal
        // dispatcher requires per-key action arms in
        // `settings_modal.rs` + `app/dispatch.rs` + `settings/registry.rs`
        // that don't yet have a place to land.
        // SHELL-owned. `restart_required: false` — the config-reloader
        // rebroadcasts UI changes; mid-session forks pick up new values.
        // Empty-string default = "no opinion" / use shell's resolution.
        SettingMeta {
            key: "fork_secondary_model",
            category: SettingCategory::Models,
            owner: SettingOwner::Shell,
            label: "分支副模型",
            description: "分支时副代理使用的模型。选「(无覆盖)」可清除。",
            keywords: &[
                "fork",
                "secondary",
                "model",
                "agent",
                "subagent",
                "branch",
                "models",
            ],
            kind: SettingKind::DynamicEnum {
                default: "",
                source: DynamicEnumSource::ActiveModelCatalog,
                supports_preview: false,
            },
            restart_required: false,
            hidden_in_minimal: false,
        },
    ]
}
