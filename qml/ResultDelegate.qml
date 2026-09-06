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
    color: root.selected ? Theme.surface1 : "transparent"
    radius: Theme.radiusRow
    border.width: 1
    border.color: root.selected ? Theme.accent : "transparent"

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceMd
        anchors.rightMargin: Theme.spaceMd
        spacing: Theme.spaceMd

        IconImage {
            implicitSize: Theme.iconSize
            source: Quickshell.iconPath(root.iconName, "application-x-executable")
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs

            Text {
                Layout.fillWidth: true
                color: Theme.text
                font.family: Theme.fontFamily
                font.pixelSize: Theme.fontName
                font.weight: Font.Medium
                elide: Text.ElideRight
                text: root.name
            }

            Text {
                Layout.fillWidth: true
                visible: root.description !== ""
                color: Theme.subtext1
                font.family: Theme.fontFamily
                font.pixelSize: Theme.fontDescription
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
