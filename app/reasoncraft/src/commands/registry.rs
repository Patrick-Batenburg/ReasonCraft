//! The list of commands, in ribbon order.

use reasoncraft_core::{
    Acceptance, CardKind, CardStrength, ClearScope, ConnectorStyle, LayoutDirection, SourceRating,
    SourceType,
};

use super::Command;
use crate::actions::{Action, Dir, Panel, Rate};
use crate::export::ExportFormat;
use crate::settings::MinimapMode;
use crate::theme::ThemeChoice;

pub(super) fn build() -> Vec<Command> {
    use Action as A;
    let cmd = |id, label, group, action| Command {
        id,
        label,
        group,
        action,
    };
    let file = "ribbon.tab.file";
    let mut out = vec![
        cmd("file.new", "action.new", file, A::New),
        cmd("file.open", "action.open", file, A::Open),
        cmd("file.save", "action.save", file, A::Save),
        cmd("file.save_as", "action.save_as", file, A::SaveAs),
    ];

    let add = "group.add";
    out.extend([
        cmd("home.add_reason", "action.add_reason", add, A::AddReason),
        cmd(
            "home.add_objection",
            "action.add_objection",
            add,
            A::AddObjection,
        ),
        cmd(
            "home.add_co_premise",
            "action.add_co_premise",
            add,
            A::AddCoPremise,
        ),
        cmd(
            "home.add_category",
            "action.add_category",
            add,
            A::AddCategory,
        ),
        cmd("home.add_note", "action.add_note", add, A::AddNote),
    ]);
    // Built once for the program's lifetime (`all` keeps them in a static).
    let leak = |s: String| -> &'static str { Box::leak(s.into_boxed_str()) };
    for st in SourceType::ALL {
        let key = st.key();
        out.push(cmd(
            leak(super::add_source_id(&st)),
            leak(format!("source.{key}")),
            "group.add_source",
            A::AddSource(st),
        ));
    }
    out.extend([
        cmd(
            "home.whole_card_reason",
            "command.whole_card_reason",
            add,
            A::TargetWholeCard(CardKind::Reason),
        ),
        cmd(
            "home.whole_card_objection",
            "command.whole_card_objection",
            add,
            A::TargetWholeCard(CardKind::Objection),
        ),
        cmd("home.comment", "action.comment", add, A::CommentSelected),
    ]);

    let editing = "group.editing";
    out.extend([
        cmd("home.edit", "command.edit", editing, A::Edit),
        cmd("home.delete", "action.delete", editing, A::Delete),
        cmd(
            "home.delete_branch",
            "action.delete_branch",
            editing,
            A::DeleteBranch,
        ),
        cmd("home.connect", "action.connect", editing, A::StartConnect),
        cmd(
            "home.jump_parking",
            "command.jump_parking",
            editing,
            A::JumpParking,
        ),
        cmd(
            "home.move_left",
            "command.move_left",
            editing,
            A::Reorder(Dir::Left),
        ),
        cmd(
            "home.move_right",
            "command.move_right",
            editing,
            A::Reorder(Dir::Right),
        ),
        cmd("home.undo", "action.undo", editing, A::Undo),
        cmd("home.redo", "action.redo", editing, A::Redo),
        cmd(
            "home.find",
            "action.find",
            editing,
            A::OpenFind { replace: false },
        ),
        cmd(
            "home.replace",
            "action.replace",
            editing,
            A::OpenFind { replace: true },
        ),
        cmd("home.find_next", "command.find_next", editing, A::FindNext),
        cmd("home.find_prev", "command.find_prev", editing, A::FindPrev),
    ]);

    let bx = "group.box";
    out.extend([
        cmd("home.link", "action.link", bx, A::EditLink),
        cmd("home.image", "action.image", bx, A::InsertImage),
        cmd(
            "home.remove_image",
            "action.remove_image",
            bx,
            A::RemoveImage,
        ),
    ]);

    out.extend(ratings());
    out.extend(view());
    out.extend(export());
    out
}

fn ratings() -> Vec<Command> {
    use Action as A;
    let cmd = |id, label, group, action| Command {
        id,
        label,
        group,
        action,
    };
    let (claim, card, source) = ("group.eval_claim", "group.eval_card", "group.eval_source");
    let accept = |a| A::SetAcceptance(Some(a));
    let strength = |s| A::SetStrength(Some(s));
    let rating = |r| A::SetSourceRating(Some(r));
    let rate = "group.evaluation";
    let display = "group.eval_view";
    vec![
        cmd(
            "evaluate.accept",
            "eval.accept",
            claim,
            accept(Acceptance::Accept),
        ),
        cmd(
            "evaluate.unsure",
            "eval.unsure",
            claim,
            accept(Acceptance::Unsure),
        ),
        cmd(
            "evaluate.reject",
            "eval.reject",
            claim,
            accept(Acceptance::Reject),
        ),
        cmd(
            "evaluate.strong",
            "eval.strong",
            card,
            strength(CardStrength::Strong),
        ),
        cmd(
            "evaluate.weak",
            "eval.weak",
            card,
            strength(CardStrength::Weak),
        ),
        cmd(
            "evaluate.card_nil",
            "eval.nil",
            card,
            strength(CardStrength::Nil),
        ),
        cmd(
            "evaluate.solid",
            "eval.solid",
            source,
            rating(SourceRating::Solid),
        ),
        cmd(
            "evaluate.shaky",
            "eval.shaky",
            source,
            rating(SourceRating::Shaky),
        ),
        cmd(
            "evaluate.source_nil",
            "eval.nil",
            source,
            rating(SourceRating::Nil),
        ),
        cmd(
            "evaluate.rate_high",
            "command.rate_high",
            rate,
            A::Rate(Rate::High),
        ),
        cmd(
            "evaluate.rate_mid",
            "command.rate_mid",
            rate,
            A::Rate(Rate::Mid),
        ),
        cmd(
            "evaluate.rate_low",
            "command.rate_low",
            rate,
            A::Rate(Rate::Low),
        ),
        cmd(
            "evaluate.rate_clear",
            "command.rate_clear",
            rate,
            A::Rate(Rate::Clear),
        ),
        cmd(
            "evaluate.clear_map",
            "eval.clear_map",
            "group.eval_clear",
            A::ClearEvaluation(ClearScope::Map),
        ),
        cmd(
            "evaluate.markers",
            "eval.show_markers",
            display,
            A::ToggleMarkers,
        ),
        cmd(
            "evaluate.shading",
            "eval.shading",
            display,
            A::ToggleShading,
        ),
    ]
}

fn view() -> Vec<Command> {
    use Action as A;
    let cmd = |id, label, group, action| Command {
        id,
        label,
        group,
        action,
    };
    let (zoom, theme, panels) = ("group.zoom", "group.theme", "group.panels");
    let direction = |id, d: LayoutDirection| {
        let label = match d {
            LayoutDirection::TopDown => "layout_direction.top_down",
            LayoutDirection::BottomUp => "layout_direction.bottom_up",
            LayoutDirection::LeftRight => "layout_direction.left_right",
        };
        cmd(id, label, "group.layout", A::SetLayoutDirection(d))
    };
    let connector = |id, s: ConnectorStyle| {
        let label = match s {
            ConnectorStyle::Curved => "connector_style.curved",
            ConnectorStyle::Elbow => "connector_style.elbow",
            ConnectorStyle::Tapered => "connector_style.tapered",
        };
        cmd(id, label, "group.layout", A::SetConnectorStyle(s))
    };
    let overview = "group.overview";
    let minimap = |id, m: MinimapMode| {
        let label = match m {
            MinimapMode::Auto => "minimap.auto",
            MinimapMode::On => "minimap.on",
            MinimapMode::Off => "minimap.off",
        };
        cmd(id, label, overview, A::SetMinimap(m))
    };
    #[allow(unused_mut)] // Debug builds add their commands.
    let mut out = vec![
        cmd("view.zoom_fit", "action.zoom_fit", zoom, A::ZoomFit),
        cmd("view.zoom_100", "action.zoom_100", zoom, A::Zoom100),
        cmd(
            "view.zoom_fit_selection",
            "action.zoom_fit_selection",
            zoom,
            A::ZoomFitSelection,
        ),
        cmd("view.zoom_in", "action.zoom_in", zoom, A::ZoomIn),
        cmd("view.zoom_out", "action.zoom_out", zoom, A::ZoomOut),
        cmd(
            "view.theme.system",
            "theme.system",
            theme,
            A::SetTheme(ThemeChoice::System),
        ),
        cmd(
            "view.theme.light",
            "theme.light",
            theme,
            A::SetTheme(ThemeChoice::Light),
        ),
        cmd(
            "view.theme.dark",
            "theme.dark",
            theme,
            A::SetTheme(ThemeChoice::Dark),
        ),
        cmd(
            "view.panel_left",
            "action.panel_left",
            panels,
            A::TogglePanel(Panel::Left),
        ),
        cmd(
            "view.panel_right",
            "action.panel_right",
            panels,
            A::TogglePanel(Panel::Right),
        ),
        direction("view.direction.top_down", LayoutDirection::TopDown),
        direction("view.direction.bottom_up", LayoutDirection::BottomUp),
        direction("view.direction.left_right", LayoutDirection::LeftRight),
        connector("view.connector.curved", ConnectorStyle::Curved),
        connector("view.connector.elbow", ConnectorStyle::Elbow),
        connector("view.connector.tapered", ConnectorStyle::Tapered),
        minimap("view.minimap.auto", MinimapMode::Auto),
        minimap("view.minimap.on", MinimapMode::On),
        minimap("view.minimap.off", MinimapMode::Off),
        cmd("view.numbers", "action.numbers", overview, A::ToggleNumbers),
    ];
    #[cfg(debug_assertions)]
    out.extend([
        cmd(
            "view.load_sample",
            "action.load_sample",
            "group.debug",
            A::LoadSampleMap,
        ),
        cmd(
            "view.source_icons",
            "action.source_icons",
            "group.debug",
            A::ToggleSourceIcons,
        ),
    ]);
    out
}

fn export() -> Vec<Command> {
    use Action as A;
    use ExportFormat as F;
    let cmd = |id, label, group, action| Command {
        id,
        label,
        group,
        action,
    };
    let format = |id, group, f: ExportFormat| cmd(id, f.name_key(), group, A::OpenExport(f));
    let (image, outline, essay) = ("group.image", "export.outline", "export.essay");
    vec![
        cmd("export.png", "export.png", image, A::OpenExport(F::Png)),
        cmd("export.svg", "export.svg", image, A::OpenExport(F::Svg)),
        cmd("export.pdf", "export.pdf", image, A::OpenExport(F::Pdf)),
        cmd(
            "export.copy_image",
            "action.copy_image",
            image,
            A::CopyAsImage,
        ),
        format("export.outline_md", outline, F::OutlineMarkdown),
        format("export.outline_txt", outline, F::OutlineText),
        format("export.outline_docx", outline, F::OutlineDocx),
        format("export.essay_md", essay, F::EssayMarkdown),
        format("export.essay_txt", essay, F::EssayText),
        format("export.essay_docx", essay, F::EssayDocx),
        cmd(
            "export.page_setup",
            "export.page_setup",
            "group.print",
            A::OpenPageSetup,
        ),
        cmd("export.print", "export.print", "group.print", A::Print),
    ]
}
