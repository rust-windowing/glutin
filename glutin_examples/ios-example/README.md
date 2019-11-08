# GlutinExample.app

## Building with `make`
* `make cargo xcode` will build the xcode project and put it in `build/Build/Products/Debug-iphonesimulator/GlutinExample.app`
* `make run` install and run the app in a simulator that's booted.
* `make simulator-logs` will give you way too many logs from the simulator.

## Building with xcode buttons
* `cd rust && make`
* `open GlutinExample.xcodeproj` and push the play button.
