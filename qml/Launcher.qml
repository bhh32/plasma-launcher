pragma ComponentBehavior: Bound

import Quickshell
import Quickshell.Wayland
import Quickshell.Widgets
import QtQuick
import QtQuick.Layouts

PanelWindow {
    id: root

    function open(): void {
        input.clear();
        Service.search("");
        root.visible = true;
    }

    function close(): void {
        root.visible = false;
    }

    function activate(index: int): void {
        const result = Service.results[index];
        if (result === undefined) return;

        Service.activate(result.id);
    }

    function complete(index: int): void {
        const result = Service.results[index];
        if (result === undefined) return;

        Service.complete(result.id);
    }

    visible: false
    color: "transparent"
    exclusionMode: ExclusionMode.Normal
    implicitWidth: Theme.cardWidth
    implicitHeight: card.maximumHeight
    mask: Region { item: card }
    anchors.top: true
    margins.top: Theme.topMargin

    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
    WlrLayershell.namespace: "plasma-launcher"

    onVisibleChanged: if (visible) input.forceActiveFocus()

    Connections {
        target: Service

        function onCloseRequested(): void {
            root.close();
        }

        function onFillRequested(text: string): void {
            input.text = text;
        }
    }

    Rectangle {
        id: card

        readonly property int chrome: Theme.spaceMd * 2 + well.implicitHeight
        readonly property int maximumHeight: card.chrome + results.headerHeight + Theme.visibleRows * Theme.rowHeight

        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right

        height: card.chrome + (list.count > 0
            ? results.headerHeight + Math.min(list.count, Theme.visibleRows) * Theme.rowHeight
            : 0)

        clip: true
        color: Theme.base
        radius: Theme.radiusCard
        border.width: 1
        border.color: Theme.surface1

        Behavior on height {
            NumberAnimation {
                duration: 110
                easing.type: Easing.OutCubic
            }
        }
        // The one motion moment: it answers the keypress that opened this
        opacity: root.visible ? 1 : 0

        Behavior on opacity {
            NumberAnimation {
                duration: 120
                easing.type: Easing.OutCubic
            }
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: Theme.spaceMd
            spacing: 0

            Rectangle {
                id: well

                Layout.fillWidth: true
                implicitHeight: field.implicitHeight + Theme.spaceSm * 2
                color: Theme.mantle
                radius: Theme.radiusField
                border.width: 1
                border.color: Theme.surface0

                RowLayout {
                    id: field

                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.leftMargin: Theme.spaceMd
                    anchors.rightMargin: Theme.spaceMd
                    spacing: Theme.spaceMd

                    IconImage {
                        implicitSize: 16
                        source: Quickshell.iconPath("system-search", "edit-find")
                    }

                    TextInput {
                        id: input

                        Layout.fillWidth: true
                        color: Theme.text
                        font.family: Theme.fontFamily
                        font.pixelSize: Theme.fontInput
                        selectionColor: Theme.surface1
                        selectedTextColor: Theme.text
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

                            color: Theme.subtext0
                            font: input.font
                            text: ""
                        }
                    }
                }
            }

            Item {
                id: results

                readonly property int headerHeight: Theme.spaceSm * 2 + 1

                Layout.fillWidth: true
                Layout.fillHeight: true

                clip: true

                Rectangle {
                    anchors.top: parent.top
                    anchors.topMargin: Theme.spaceSm
                    anchors.left: parent.left
                    anchors.right: parent.right
                    height: 1
                    color: Theme.surface0
                }

                ListView {
                    id: list

                    anchors.top: parent.top
                    anchors.topMargin: results.headerHeight
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom

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
}
