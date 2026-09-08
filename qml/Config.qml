pragma Singleton

import Quickshell
import Quickshell.Io

Singleton {
    id: root

    readonly property string binary: "foothold-config"

    property var appearance: ({})
    property var paths: ({})

    Process {
        id: reader

        command: [root.binary]
        running: true
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    const config = JSON.parse(this.text);
                    root.appearance = config.appearance;
                    root.paths = config.paths;
                } catch (error) {
                    console.warn("could not read config:", error);
                }
            }
        }
        stderr: SplitParser {
            onRead: data => console.warn("config:", data)
        }
    }

    // Both paths arrive from the first run, so these bind after it and
    // re-run the reader whenever either file changes on disk
    FileView {
        path: root.paths.config ?? ""
        watchChanges: true
        onFileChanged: reader.running = true
    }

    FileView {
        path: root.paths.desktop ?? ""
        watchChanges: true
        onFileChanged: reader.running = true
    }
}
