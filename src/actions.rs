use gpui::actions;

actions!(
    keel,
    [
        OpenProject,
        SaveFile,
        CloseEditor,
        ToggleProjectPanel,
        ToggleBottomPanel,
        ShowCommandPalette,
        PaletteConfirm,
        PaletteCancel,
        PaletteUp,
        PaletteDown,
    ]
);
