use nu_command::*;
use nu_protocol::engine::{EngineState, StateWorkingSet};

use std::path::Path;

// We maintain our own default context so that we can control the commands supported,
// and prevent name clashes. This function should be very similar to the same one in nu_command.
pub fn create_default_context(cwd: impl AsRef<Path>) -> EngineState {
    let mut engine_state = EngineState::new();

    let delta = {
        let mut working_set = StateWorkingSet::new(&engine_state);

        macro_rules! bind_command {
            ( $( $command:expr ),* $(,)? ) => {
                $( working_set.add_decl(Box::new($command)); )*
            };
        }

        // If there are commands that have the same name as default declarations,
        // they have to be registered before the main declarations. This helps to make
        // them only accessible if the correct input value category is used with the
        // declaration.
        // These commands typically all start with dfr so we're safe to blindly add them all.
        add_dataframe_decls(&mut working_set);

        // Core
        bind_command! {
            Alias,
            Debug,
            Def,
            DefEnv,
            Describe,
            Do,
            Du,
            Echo,
            ErrorMake,
            ExportCommand,
            ExportDef,
            ExportDefEnv,
            ExportEnv,
            Extern,
            For,
            // Help,
            Hide,
            History,
            If,
            Ignore,
            Let,
            Metadata,
            Module,
            Source,
            Tutor,
            Use,
            // Version,
        };

        // Filters
        bind_command! {
            All,
            Any,
            Append,
            Collect,
            Columns,
            Compact,
            Default,
            Drop,
            DropColumn,
            DropNth,
            Each,
            Empty,
            Every,
            Find,
            First,
            Flatten,
            Get,
            Group,
            GroupBy,
            Headers,
            SplitBy,
            Take,
            Merge,
            Move,
            TakeUntil,
            TakeWhile,
            Last,
            Length,
            Lines,
            ParEach,
            Prepend,
            Range,
            Reduce,
            Reject,
            Rename,
            Reverse,
            Roll,
            RollDown,
            RollUp,
            RollLeft,
            RollRight,
            Rotate,
            Select,
            Shuffle,
            Skip,
            SkipUntil,
            SkipWhile,
            SortBy,
            Transpose,
            Uniq,
            Update,
            UpdateCells,
            Where,
            Window,
            Wrap,
            Zip,
        };

        // Path
        bind_command! {
            Path,
            PathBasename,
            PathDirname,
            PathExists,
            PathExpand,
            PathJoin,
            PathParse,
            PathRelativeTo,
            PathSplit,
            PathType,
        };

        // System
        bind_command! {
            Benchmark,
            Complete,
            Exec,
            External,
            Ps,
            Sys,
        };

        bind_command! { Which };

        // Strings
        bind_command! {
            BuildString,
            Char,
            Decode,
            DetectColumns,
            Format,
            Parse,
            Size,
            Split,
            SplitChars,
            SplitColumn,
            SplitRow,
            Str,
            StrCamelCase,
            StrCapitalize,
            StrCollect,
            StrContains,
            StrDowncase,
            StrEndswith,
            StrReplace,
            StrIndexOf,
            StrKebabCase,
            StrLength,
            StrLpad,
            StrPascalCase,
            StrReverse,
            StrRpad,
            StrScreamingSnakeCase,
            StrSnakeCase,
            StrStartsWith,
            StrSubstring,
            StrTrim,
            StrUpcase
        };

        // FileSystem
        bind_command! {
            Cd,
            Cp,
            Ls,
            Mkdir,
            Mv,
            Open,
            Rm,
            Save,
            Touch,
        };

        // Platform
        bind_command! {
            Ansi,
            AnsiGradient,
            AnsiStrip,
            Clear,
            KeybindingsDefault,
            Input,
            KeybindingsListen,
            Keybindings,
            Kill,
            KeybindingsList,
            Sleep,
            TermSize,
        };

        // Date
        bind_command! {
            Date,
            DateFormat,
            DateHumanize,
            DateListTimezones,
            DateNow,
            DateToTable,
            DateToTimezone,
        };

        // Shells
        bind_command! {
            Enter,
            Exit,
            GotoShell,
            NextShell,
            PrevShell,
            Shells,
        };

        // Formats
        bind_command! {
            From,
            FromCsv,
            FromEml,
            FromIcs,
            FromIni,
            FromJson,
            FromNuon,
            FromOds,
            FromSsv,
            FromToml,
            FromTsv,
            FromUrl,
            FromVcf,
            FromXlsx,
            FromXml,
            FromYaml,
            FromYml,
            To,
            ToCsv,
            ToHtml,
            ToJson,
            ToMd,
            ToNuon,
            ToToml,
            ToTsv,
            ToCsv,
            Touch,
            Use,
            Update,
            Where,
            ToUrl,
            ToXml,
            ToYaml,
        };

        // Viewers
        bind_command! {
            Griddle,
            Table,
        };

        // Conversions
        bind_command! {
            Fmt,
            Into,
            IntoBool,
            IntoBinary,
            IntoDatetime,
            IntoDecimal,
            IntoDuration,
            IntoFilesize,
            IntoInt,
            IntoString,
        };

        // Env
        bind_command! {
            Env,
            LetEnv,
            LoadEnv,
            WithEnv,
        };

        // Math
        bind_command! {
            Math,
            MathAbs,
            MathAvg,
            MathCeil,
            MathEval,
            MathFloor,
            MathMax,
            MathMedian,
            MathMin,
            MathMode,
            MathProduct,
            MathRound,
            MathSqrt,
            MathStddev,
            MathSum,
            MathVariance,
        };

        // Network
        bind_command! {
            Fetch,
            Post,
            Url,
            UrlHost,
            UrlPath,
            UrlQuery,
            UrlScheme,
        }

        // Random
        bind_command! {
            Random,
            RandomBool,
            RandomChars,
            RandomDecimal,
            RandomDice,
            RandomInteger,
            RandomUuid,
        };

        // Generators
        bind_command! {
            Cal,
            Seq,
            SeqDate,
        };

        // Hash
        bind_command! {
            Hash,
            HashMd5::default(),
            HashSha256::default(),
            Base64,
        };

        // Experimental
        bind_command! {
            ViewSource,
        };

        bind_command!(Register);

        working_set.render()
    };

    let _ = engine_state.merge_delta(delta, None, &cwd);

    engine_state
}