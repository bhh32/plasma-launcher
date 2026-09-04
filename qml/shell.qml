import Quickshell
import Quickshell.Io

ShellRoot {
    Launcher {
        id: launcher
    }

    IpcHandler {
        target: "launcher"

        function toggle(): void {
            if (launcher.visible) launcher.close();
            else launcher.open();
        }

        function show(): void {
            launcher.open();
        }

        function hide(): void {
            launcher.close();
        }
    }
}
