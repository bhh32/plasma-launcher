pragma Singleton

import Quickshell
import Quickshell.Io

Singleton {
    id: root

    // Dev path
    readonly property string binary: "/home/bryan/Projects/rust/personal/plasma-launcher/target/debug/plasma-launcher"
    property var results: []

    signal closeRequested()
    signal fillRequested(string text)

    function search(query: string): void {
        root.send({ Search: query });
    }

    function activate(id: int): void {
        root.send({ Activate: id });
    }

    function complete(id: int): void {
        root.send({ Complete: id });
    }

    function send(message): void {
        if (!backend.running) {
            console.warn("dropping a request, the backend is not running");
            return;
        }

        backend.write(JSON.stringify(message) + "\n");
    }

    function receive(line: string): void {
        let message;

        try {
            message = JSON.parse(line);
        } catch (error) {
            console.warn("unparseable line from the backend:", line);
            return;
        }

        // Unit variants arrive as bare strings, every other variant as a
        // single-key object
        if (message === "Close") {
            root.closeRequested();
            return;
        }

        if (message.Update !== undefined) {
            root.results = message.Update;
            return;
        }

        if (message.DesktopEntry !== undefined) {
            Quickshell.execDetached(["gio", "launch", message.DesktopEntry.path]);
            return;
        }

        if (message.Fill !== undefined) {
            root.fillRequested(message.Fill);
            return;
        }

        console.warn("unhandled response:", line);
    }

    Process {
        id: backend

        command: [root.binary]
        running: true
        stdinEnabled: true
        stdout: SplitParser {
            onRead: data => root.receive(data)
        }
        stderr: SplitParser {
            onRead: data => console.log("backend:", data)
        }

        onExited: (exitCode, exitStatus) => {
            root.results = [];
            console.warn(`backend exited with code ${exitCode}`);
        }
    }
}
