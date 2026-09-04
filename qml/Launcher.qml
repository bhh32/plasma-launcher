pragma ComponentBehavior: Bound

import Quickshell
import Quickshell.Wayland
import QtQuick
import QtQuick.Layouts

PanelWindow {
    id: root

    // Placeholder
    readonly property var entries: [
        { name: "Firefox", description: "Web Browser", icon: "firefox", exec: ["firefox"] },
        { name: "Konsole", description: "Terminal", icon: "utilities-terminal", exec: ["terminal"] },
        { name: "Dolphin", description: "File Manager", icon: "system-file-manager", exec: ["dolphin"] },
        { name: "System Settings", description: "Configure KDE Plasma", icon: "systemsettings", exec: ["systemsettings"] },
        { name: "Spectacle", description: "Screenshot Capture", icon: "spectacle", exec: ["spectacle"] }
    ]

    readonly property string query: input.text.trim().toLowerCase()
    readonly property var results: {
        if (root.query === "") return root.entries;
        return root.entries.filter(entry =>
            entry.name.toLowerCase().includes(root.query)
            || entry.description.toLowerCase().includes(root.query));
    }

    function open(): void {
        input.clear();
        root.visible = true;
    }

    function close(): void {
        root.visible = false;
    }

    function activate(index: int): void {
        const result = Service.results[index];
        if(result === undefined) return;

        Service.activate(result.id);
    }

    function complete(index: int): void {
        const result = Service.results[index];
        if (result === undefined) return;

        Service.complete(result.id)
    }

    visible: false
    color: "transparent"
    exclusionMode: ExclusionMode.Normal
    implicitWidth: Theme.cardWidth
    implicitHeight: layout.implicitHeight + Theme.padding * 2
    anchors.top: true
    margins.top: Theme.topMargin

    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
    WlrLayershell.namespace: "plasma-launcher"

    onVisibleChanged: if (visible) input.forceActiveFocus()

    MouseArea {
        anchors.fill: parent
        onClicked: root.close()
    }

    Rectangle {
        id: card

        anchors.fill: parent
        anchors.horizontalCenter: parent.horizontalCenter
        y: Math.round(parent.height * 0.2)
        width: Theme.cardWidth
        height: layout.implicitHeight + Theme.padding * 2
        color: Theme.surface
        radius: Theme.radius

        ColumnLayout {
            id: layout

            anchors.fill: parent
            anchors.margins: Theme.padding
            spacing: Theme.padding

            TextInput {
                id: input

                Layout.fillWidth: true
                color: Theme.text
                font.pixelSize: 18
                selectionColor: Theme.accent
                selectedTextColor: Theme.base
                focus: true

                onAccepted: root.activate(list.currentIndex)

                onTextChanged: Service.search(text)
                Keys.onTabPressed: root.complete(list.currentIndex)
                Keys.onEscapePressed: root.close()
                Keys.onDownPressed: list.incrementCurrentIndex()
                Keys.onUpPressed: list.decrementCurrentIndex()

                Text {
                    anchors.fill: parent
                    visible: input.text === ""

                    color: Theme.subtext
                    font: input.font
                    text: "Search applications"
                }
            }

            Rectangle {
                Layout.fillWidth: true
                implicitHeight: 1
                color: Theme.overlay
                visible: list.count > 0
            }

            ListView {
                id: list

                Layout.fillWidth: true
                Layout.preferredHeight: Math.min(count, Theme.visibleRows) * Theme.rowHeight
                clip: true
                model: Service.results
                onModelChanged: currentIndex = 0
                delegate: ResultDelegate {
                    required property var modelData
                    required property int index

                    width: list.width
                    name: modelData.name
                    description: modelData.description
                    iconName: modelData.icon && modelData.icon.Name ? modelData.icon.Name : ""
                    selected: index === list.currentIndex
                    onClicked: root.activate(index)
                    onHovered: list.currentIndex = index
                }
            }
        }
    }
}
