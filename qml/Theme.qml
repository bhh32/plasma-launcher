pragma Singleton

import Quickshell
import QtQuick

Singleton {
    readonly property color base: "#232634"
    readonly property color surface: "#303446"
    readonly property color overlay: "#414559"
    readonly property color text: "#c6d0f5"
    readonly property color subtext: "#a5adce"
    readonly property color accent: "#babbf1"
    readonly property color scrim: "#99232634"

    readonly property int cardWidth: 600
    readonly property int topMargin: 16
    readonly property int rowHeight: 52
    readonly property int visibleRows: 8
    readonly property int radius: 12
    readonly property int padding: 12
}
