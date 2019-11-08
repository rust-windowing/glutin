.PHONY: xcode cargo run install simulator-logs

LIB = rust/libglutin_ios_example.a
run: install
	xcrun simctl launch booted com.glutin.GlutinExample

install: xcode
	xcrun simctl install booted build/Build/Products/Debug-iphonesimulator/GlutinExample.app

xcode: $(LIB)
	xcrun xcodebuild -scheme GlutinExample -project GlutinExample.xcodeproj/ -configuration Debug -destination 'platform=iOS Simulator,name=iPhone 11 Pro,OS=13.2' -derivedDataPath build

cargo:
	make -C rust

simulator-logs:
	xcrun simctl spawn booted log stream --level=debug --predicate 'processImagePath endswith "GlutinExample"'
ci:
	make -C rust ci
	xcrun xcodebuild -scheme GlutinExample -project GlutinExample.xcodeproj/ -configuration Debug -destination 'platform=iOS Simulator,name=iPhone 11 Pro,OS=13.2' -derivedDataPath build


