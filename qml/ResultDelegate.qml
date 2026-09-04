import Quickshell
import Quickshell.Widgets
import QtQuick
import QtQuick.Layouts

Rectangle {
    id: root

    required property string name
    required property string description
    required property string iconName

    property bool selected: false

    signal clicked()
    signal hovered()

    implicitHeight: Theme.rowHeight

    color: root.selected ? Theme.overlay : "transparent"
    radius: Theme.radius / 2

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.padding
        anchors.rightMargin: Theme.padding
        spacing: Theme.padding

        IconImage {
            implicitSize: 32
            source: Quickshell.iconPath(root.iconName, "application-x-executable")
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0

            Text {
                Layout.fillWidth: true
                color: Theme.text
                font.pixelSize: 14
                elide: Text.ElideRight
                text: root.name
            }

            Text {
                Layout.fillWidth: true
                visible: root.description !== ""
                color: Theme.subtext
                font.pixelSize: 11
                elide: Text.ElideRight
                text: root.description
            }
        }
    }

    MouseArea {
        anchors.fill: parent
        hoverEnabled: true
        onEntered: root.hovered()
        onClicked: root.clicked()
    }
}
