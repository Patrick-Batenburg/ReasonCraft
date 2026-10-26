//! The Export tab's commands: images, text formats and printing.

use super::cmd;
use crate::actions::Action;
use crate::commands::Command;
use crate::export::ExportFormat;

pub(super) fn commands() -> Vec<Command> {
    use Action as A;
    use ExportFormat as F;
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
            "export.argdown",
            "export.argdown",
            "group.text",
            A::OpenExport(F::Argdown),
        ),
        cmd(
            "export.page_setup",
            "export.page_setup",
            "group.print",
            A::OpenPageSetup,
        ),
        cmd("export.print", "export.print", "group.print", A::Print),
    ]
}
