//! The Export tab's commands: images, text formats and printing.

use super::cmd;
use crate::actions::Action;
use crate::commands::Command;
use crate::export::ExportFormat;
use crate::shell::icons::Icon;

pub(super) fn commands() -> Vec<Command> {
    use Action as A;
    use ExportFormat as F;
    let format = |id, group, f: ExportFormat| cmd(id, f.name_key(), group, A::OpenExport(f));
    let (image, outline, essay) = ("group.image", "export.outline", "export.essay");
    vec![
        cmd("export.png", "export.png", image, A::OpenExport(F::Png)).with_icon(Icon::ExportPng),
        cmd("export.svg", "export.svg", image, A::OpenExport(F::Svg)).with_icon(Icon::ExportSvg),
        cmd("export.pdf", "export.pdf", image, A::OpenExport(F::Pdf)).with_icon(Icon::ExportPdf),
        cmd(
            "export.copy_image",
            "action.copy_image",
            image,
            A::CopyAsImage,
        )
        .with_icon(Icon::CopyImage),
        format("export.outline_md", outline, F::OutlineMarkdown),
        format("export.outline_txt", outline, F::OutlineText),
        format("export.outline_docx", outline, F::OutlineDocx),
        format("export.essay_md", essay, F::EssayMarkdown),
        format("export.essay_txt", essay, F::EssayText),
        format("export.essay_docx", essay, F::EssayDocx),
        cmd(
            "export.argdown",
            "export.argdown",
            "group.text",
            A::OpenExport(F::Argdown),
        )
        .with_icon(Icon::Argdown),
        cmd(
            "export.page_setup",
            "export.page_setup",
            "group.print",
            A::OpenPageSetup,
        )
        .with_icon(Icon::PageSetup),
        cmd("export.print", "export.print", "group.print", A::Print).with_icon(Icon::Print),
    ]
}
