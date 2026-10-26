//! Central Command Registry for RustPad's toolbars, shortcuts, and `Ctrl+K` Command Palette.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandId {
    NewWordBlank,
    NewWordWelcome,
    NewPdfBlank,
    OpenFile,
    SaveFile,
    SaveAsFile,
    ExportWordToPdf,
    ConvertPdfToWord,
    ToggleBold,
    ToggleItalic,
    ToggleUnderline,
    ToggleStrike,
    CycleStyle,
    CycleFontFamily,
    FontSizeUp,
    FontSizeDown,
    CycleTextColor,
    ToggleHighlight,
    AlignLeft,
    AlignCenter,
    AlignRight,
    ToggleBulletList,
    ToggleChecklist,
    InsertTable,
    TableAddRow,
    TableAddCol,
    InsertPageBreak,
    CyclePaperSize,
    ToggleSpellcheck,
    PdfToolSelect,
    PdfToolAddText,
    PdfToolHighlight,
    PdfToolSticky,
    PdfToolRedact,
    OpenSignaturePad,
    StampApproved,
    PdfMergeFile,
    PdfSplitModal,
    PdfCompress,
    PdfRotatePage,
    PdfAddBlankPage,
    PdfDeletePage,
    OpenWatermarkModal,
    ToggleSplitView,
    OpenSnapshotsDrawer,
    CaptureSnapshot,
    OpenFindReplace,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    OpenAbout,
}

pub struct CommandSpec {
    pub id: CommandId,
    pub category: &'static str,
    pub title: &'static str,
    pub shortcut: &'static str,
}

pub const ALL_COMMANDS: &[CommandSpec] = &[
    CommandSpec { id: CommandId::NewWordBlank, category: "File", title: "New Blank Word Document (.docx)", shortcut: "Ctrl+N" },
    CommandSpec { id: CommandId::NewWordWelcome, category: "File", title: "New Document from Welcome Guide", shortcut: "" },
    CommandSpec { id: CommandId::NewPdfBlank, category: "PDF", title: "Create New Blank PDF Studio Tab (.pdf)", shortcut: "" },
    CommandSpec { id: CommandId::OpenFile, category: "File", title: "Open Word / PDF / Markdown File…", shortcut: "Ctrl+O" },
    CommandSpec { id: CommandId::SaveFile, category: "File", title: "Save Active Document", shortcut: "Ctrl+S" },
    CommandSpec { id: CommandId::SaveAsFile, category: "File", title: "Save Active Document As…", shortcut: "" },
    CommandSpec { id: CommandId::ExportWordToPdf, category: "Convert", title: "Export Word Document to Vector PDF (.pdf)", shortcut: "Ctrl+E" },
    CommandSpec { id: CommandId::ConvertPdfToWord, category: "Convert", title: "Convert Active PDF to Editable Word (.docx)", shortcut: "" },
    CommandSpec { id: CommandId::ToggleBold, category: "Word", title: "Toggle Bold Formatting", shortcut: "Ctrl+B" },
    CommandSpec { id: CommandId::ToggleItalic, category: "Word", title: "Toggle Italic Formatting", shortcut: "Ctrl+I" },
    CommandSpec { id: CommandId::ToggleUnderline, category: "Word", title: "Toggle Underline Formatting", shortcut: "Ctrl+U" },
    CommandSpec { id: CommandId::ToggleStrike, category: "Word", title: "Toggle Strikethrough", shortcut: "" },
    CommandSpec { id: CommandId::CycleStyle, category: "Word", title: "Cycle Paragraph Style (Normal / H1 / H2 / Title / Quote)", shortcut: "" },
    CommandSpec { id: CommandId::CycleFontFamily, category: "Word", title: "Switch Font Family (Sans / Serif / Mono)", shortcut: "" },
    CommandSpec { id: CommandId::FontSizeUp, category: "Word", title: "Increase Font Size", shortcut: "" },
    CommandSpec { id: CommandId::FontSizeDown, category: "Word", title: "Decrease Font Size", shortcut: "" },
    CommandSpec { id: CommandId::CycleTextColor, category: "Word", title: "Cycle Text Ink Color (Slate / Cobalt / Coral / Emerald)", shortcut: "" },
    CommandSpec { id: CommandId::ToggleHighlight, category: "Word", title: "Toggle Soft Marker Highlight", shortcut: "" },
    CommandSpec { id: CommandId::AlignLeft, category: "Word", title: "Align Paragraph Left", shortcut: "" },
    CommandSpec { id: CommandId::AlignCenter, category: "Word", title: "Align Paragraph Center", shortcut: "" },
    CommandSpec { id: CommandId::AlignRight, category: "Word", title: "Align Paragraph Right", shortcut: "" },
    CommandSpec { id: CommandId::ToggleBulletList, category: "Word", title: "Toggle Bulleted List", shortcut: "" },
    CommandSpec { id: CommandId::ToggleChecklist, category: "Word", title: "Toggle Interactive Checklist", shortcut: "" },
    CommandSpec { id: CommandId::InsertTable, category: "Word", title: "Insert Styled 3×3 Table", shortcut: "" },
    CommandSpec { id: CommandId::TableAddRow, category: "Word", title: "Table: Add Row Below", shortcut: "" },
    CommandSpec { id: CommandId::TableAddCol, category: "Word", title: "Table: Add Column Right", shortcut: "" },
    CommandSpec { id: CommandId::InsertPageBreak, category: "Word", title: "Insert Physical Page Break", shortcut: "" },
    CommandSpec { id: CommandId::CyclePaperSize, category: "Word", title: "Switch Paper Size (A4 / US Letter / US Legal)", shortcut: "" },
    CommandSpec { id: CommandId::ToggleSpellcheck, category: "Tools", title: "Toggle Offline Live Spellchecker", shortcut: "" },
    CommandSpec { id: CommandId::OpenSignaturePad, category: "PDF", title: "Draw & Place Ink Signature on PDF", shortcut: "" },
    CommandSpec { id: CommandId::StampApproved, category: "PDF", title: "Place [APPROVED] Vector Stamp on PDF", shortcut: "" },
    CommandSpec { id: CommandId::PdfToolAddText, category: "PDF", title: "PDF Tool: Click Page to Add Text Box", shortcut: "" },
    CommandSpec { id: CommandId::PdfToolHighlight, category: "PDF", title: "PDF Tool: Click Page to Add Highlight", shortcut: "" },
    CommandSpec { id: CommandId::PdfToolRedact, category: "PDF", title: "PDF Tool: True Security Redaction Box", shortcut: "" },
    CommandSpec { id: CommandId::PdfMergeFile, category: "PDF", title: "Merge Another PDF into Current PDF…", shortcut: "Ctrl+M" },
    CommandSpec { id: CommandId::PdfSplitModal, category: "PDF", title: "Split / Extract Page Ranges from PDF…", shortcut: "" },
    CommandSpec { id: CommandId::PdfCompress, category: "PDF", title: "Compress & Optimize Active PDF Streams", shortcut: "" },
    CommandSpec { id: CommandId::PdfRotatePage, category: "PDF", title: "Rotate Active PDF Page 90°", shortcut: "" },
    CommandSpec { id: CommandId::PdfAddBlankPage, category: "PDF", title: "Insert Blank Page into PDF", shortcut: "" },
    CommandSpec { id: CommandId::OpenWatermarkModal, category: "Tools", title: "Set / Clear Diagonal Page Watermark…", shortcut: "" },
    CommandSpec { id: CommandId::ToggleSplitView, category: "View", title: "Toggle Side-by-Side Split Workspace", shortcut: "Ctrl+\\" },
    CommandSpec { id: CommandId::OpenSnapshotsDrawer, category: "Tools", title: "Open Local Time-Machine Version History", shortcut: "" },
    CommandSpec { id: CommandId::CaptureSnapshot, category: "Tools", title: "Capture Local Version Snapshot Now", shortcut: "" },
    CommandSpec { id: CommandId::OpenFindReplace, category: "Tools", title: "Find & Replace in Document…", shortcut: "Ctrl+F" },
    CommandSpec { id: CommandId::ZoomIn, category: "View", title: "Zoom In (+10%)", shortcut: "Ctrl+=" },
    CommandSpec { id: CommandId::ZoomOut, category: "View", title: "Zoom Out (-10%)", shortcut: "Ctrl+-" },
    CommandSpec { id: CommandId::ZoomReset, category: "View", title: "Reset Zoom to 100%", shortcut: "Ctrl+0" },
    CommandSpec { id: CommandId::OpenAbout, category: "Help", title: "About RustPad & Author Info", shortcut: "" },
];

pub fn filter_commands(query: &str) -> Vec<&'static CommandSpec> {
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return ALL_COMMANDS.iter().collect();
    }
    let terms: Vec<&str> = q.split_whitespace().collect();
    ALL_COMMANDS
        .iter()
        .filter(|cmd| {
            let hay = format!(
                "{} {} {}",
                cmd.category.to_ascii_lowercase(),
                cmd.title.to_ascii_lowercase(),
                cmd.shortcut.to_ascii_lowercase()
            );
            terms.iter().all(|t| hay.contains(t))
        })
        .collect()
}
