pragma Singleton

import Quickshell
import QtQuick

Singleton {
    id: root

    readonly property var colors: Config.appearance.colors ?? ({})
    readonly property var radii: Config.appearance.radius ?? ({})
    readonly property var fonts: Config.appearance.font ?? ({})

    readonly property color base: root.colors.base ?? "#303446"
    readonly property color mantle: root.colors.mantle ?? "#292c3c"
    readonly property color surface0: root.colors.surface0 ?? "#414559"
    readonly property color surface1: root.colors.surface1 ?? "#51576d"
    readonly property color text: root.colors.text ?? "#c6d0f5"
    readonly property color subtext0: root.colors.subtext0 ?? "#a5adce"
    readonly property color subtext1: root.colors.subtext1 ?? "#b5bfe2"
    readonly property color accent: root.colors.accent ?? "#babbf1"

    readonly property int cardWidth: Config.appearance.card_width ?? 600
    readonly property int topMargin: Config.appearance.top_margin ?? 16
    readonly property int rowHeight: Config.appearance.row_height ?? 56
    readonly property int visibleRows: Config.appearance.visible_rows ?? 8
    readonly property int iconSize: Config.appearance.icon_size ?? 32

    readonly property int radiusCard: root.radii.card ?? 16
    readonly property int radiusField: root.radii.field ?? 10
    readonly property int radiusRow: root.radii.row ?? 8

    readonly property string fontFamily: {
        const configured = root.fonts.family ?? "";
        return configured !== "" ? configured: Qt.application.font.family;
    }
    readonly property int fontInput: root.fonts.input ?? 19
    readonly property int fontName: root.fonts.name ?? 15
    readonly property int fontDescription: root.fonts.description ?? 12

    // Spacing is rhythm, not preference, so it stays out of the Config
    readonly property int spaceXs: 2
    readonly property int spaceSm: 8
    readonly property int spaceMd: 12
    readonly property int spaceLg: 20
}
