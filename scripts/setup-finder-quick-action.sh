#!/usr/bin/env bash
#
# Install macOS Finder Right-Click Quick Action ("Convert with UFC")
#
set -euo pipefail

SERVICES_DIR="$HOME/Library/Services"
WORKFLOW_NAME="Convert with UFC.workflow"
TARGET_DIR="$SERVICES_DIR/$WORKFLOW_NAME"
CONTENTS_DIR="$TARGET_DIR/Contents"

echo "==> Creating macOS Finder Quick Action: '$WORKFLOW_NAME' ..."
mkdir -p "$CONTENTS_DIR"

# 1. Write Info.plist
cat <<'EOF' > "$CONTENTS_DIR/Info.plist"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>NSServices</key>
	<array>
		<dict>
			<key>NSMenuItem</key>
			<dict>
				<key>default</key>
				<string>Convert with UFC</string>
			</dict>
			<key>NSMessage</key>
			<string>runWorkflowAsService</string>
			<key>NSRequiredContext</key>
			<dict>
				<key>NSApplicationIdentifier</key>
				<string>com.apple.finder</string>
			</dict>
			<key>NSSendFileTypes</key>
			<array>
				<string>public.item</string>
			</array>
		</dict>
	</array>
</dict>
</plist>
EOF

# 2. Write document.wflow (Automator Quick Action with embedded shell runner)
cat <<'EOF' > "$CONTENTS_DIR/document.wflow"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>AMApplicationBuild</key>
	<string>523</string>
	<key>AMApplicationVersion</key>
	<string>2.10</string>
	<key>AMDocumentVersion</key>
	<string>2</string>
	<key>actions</key>
	<array>
		<dict>
			<key>action</key>
			<dict>
				<key>AMAccepts</key>
				<dict>
					<key>Container</key>
					<string>List</string>
					<key>Optional</key>
					<true/>
					<key>Types</key>
					<array>
						<string>com.apple.cocoa.string</string>
					</array>
				</dict>
				<key>AMActionVersion</key>
				<string>2.0.3</string>
				<key>AMApplication</key>
				<array>
					<string>Automator</string>
				</array>
				<key>AMParameterProperties</key>
				<dict>
					<key>COMMAND_STRING</key>
					<dict/>
					<key>CheckedForUserDefaultShell</key>
					<dict/>
					<key>inputMethod</key>
					<dict/>
					<key>shell</key>
					<dict/>
					<key>source</key>
					<dict/>
				</dict>
				<key>AMProvides</key>
				<dict>
					<key>Container</key>
					<string>List</string>
					<key>Types</key>
					<array>
						<string>com.apple.cocoa.string</string>
					</array>
				</dict>
				<key>ActionBundlePath</key>
				<string>/System/Library/Automator/Run Shell Script.action</string>
				<key>ActionName</key>
				<string>Run Shell Script</string>
				<key>ActionParameters</key>
				<dict>
					<key>COMMAND_STRING</key>
					<string>export PATH="/opt/homebrew/bin:/opt/homebrew/sbin:/usr/local/bin:$HOME/.cargo/bin:$HOME/.local/bin:$PATH"

UFC_BIN=""
for candidate in "/opt/homebrew/bin/ufc" "/usr/local/bin/ufc" "$HOME/.cargo/bin/ufc" "$HOME/.local/bin/ufc"; do
    if [ -x "$candidate" ]; then
        UFC_BIN="$candidate"
        break
    fi
done
if [ -z "$UFC_BIN" ] && command -v ufc &gt;/dev/null 2&gt;&amp;1; then
    UFC_BIN="$(command -v ufc)"
fi

if [ -z "$UFC_BIN" ]; then
    osascript -e 'display dialog "ufc binary was not found. Please install Universal File Converter first." buttons {"OK"} default button "OK" with icon stop'
    exit 1
fi

RAW_FORMAT=$(osascript &lt;&lt;'APPLESCRIPT'
set formatList to {"webp", "png", "jpeg", "pdf", "txt", "yaml", "json", "toml", "csv", "html", "md", "docx", "Other..."}
set chosen to choose from list formatList with title "Universal File Converter" with prompt "Select target format:" default items {"webp"}
if chosen is false then
    return "CANCELLED"
end if
set chosenItem to item 1 of chosen
if chosenItem is "Other..." then
    set customFormat to display dialog "Enter target format extension (e.g. bmp, gif, tiff, odt):" default answer "" with title "UFC Custom Format"
    return text returned of customFormat
else
    return chosenItem
end if
APPLESCRIPT
)

TARGET_FORMAT=$(echo "$RAW_FORMAT" | tr -d '\r\n[:space:]')

if [ "$TARGET_FORMAT" = "CANCELLED" ] || [ -z "$TARGET_FORMAT" ]; then
    exit 0
fi

SUCCESS_COUNT=0
FAIL_COUNT=0
FAIL_LOG=""

for f in "$@"; do
    if [ -f "$f" ]; then
        ERR_FILE=$(mktemp)
        if "$UFC_BIN" convert "$f" --to "$TARGET_FORMAT" &gt;"$ERR_FILE" 2&gt;&amp;1; then
            ((SUCCESS_COUNT++))
        else
            ((FAIL_COUNT++))
            ERR_MSG=$(cat "$ERR_FILE" | head -n 2 | tr '"' "'")
            FAIL_LOG="${FAIL_LOG}\n• $(basename "$f"): ${ERR_MSG}"
        fi
        rm -f "$ERR_FILE"
    fi
done

if [ "$FAIL_COUNT" -eq 0 ]; then
    osascript -e "display notification \"Converted $SUCCESS_COUNT file(s) to $TARGET_FORMAT successfully.\" with title \"UFC Complete\""
else
    osascript -e "display alert \"UFC Conversion Notice\" message \"$SUCCESS_COUNT converted, $FAIL_COUNT failed.${FAIL_LOG}\" as warning"
fi
</string>

					<key>CheckedForUserDefaultShell</key>
					<true/>
					<key>inputMethod</key>
					<integer>1</integer>
					<key>shell</key>
					<string>/bin/bash</string>
					<key>source</key>
					<string></string>
				</dict>
				<key>BundleIdentifier</key>
				<string>com.apple.RunShellScript</string>
				<key>CFBundleVersion</key>
				<string>2.0.3</string>
				<key>CanShowSelectedItemsWhenRun</key>
				<false/>
				<key>CanShowWhenRun</key>
				<true/>
				<key>Category</key>
				<array>
					<string>AMCategoryUtilities</string>
				</array>
				<key>Class Name</key>
				<string>RunShellScriptAction</string>
				<key>InputUUID</key>
				<string>5A3E9B71-92D1-45D3-8E5A-7FEA50920401</string>
				<key>Keywords</key>
				<array>
					<string>Shell</string>
					<string>Script</string>
					<string>Command</string>
					<string>Run</string>
					<string>Unix</string>
				</array>
				<key>OutputUUID</key>
				<string>4E59C72F-1A2B-4C3D-8E9F-0A1B2C3D4E5F</string>
				<key>UUID</key>
				<string>3B2A1C0D-9E8F-7A6B-5C4D-3E2F1A0B9C8D</string>
				<key>UnlocalizedApplications</key>
				<array>
					<string>Automator</string>
				</array>
				<key>arguments</key>
				<dict>
					<key>0</key>
					<dict>
						<key>default value</key>
						<integer>0</integer>
						<key>name</key>
						<string>inputMethod</string>
						<key>required</key>
						<string>0</string>
						<key>type</key>
						<string>0</string>
						<key>value</key>
						<integer>1</integer>
					</dict>
					<key>1</key>
					<dict>
						<key>default value</key>
						<string></string>
						<key>name</key>
						<string>source</string>
						<key>required</key>
						<string>0</string>
						<key>type</key>
						<string>0</string>
						<key>value</key>
						<string></string>
					</dict>
					<key>2</key>
					<dict>
						<key>default value</key>
						<false/>
						<key>name</key>
						<string>CheckedForUserDefaultShell</string>
						<key>required</key>
						<string>0</string>
						<key>type</key>
						<string>0</string>
						<key>value</key>
						<true/>
					</dict>
					<key>3</key>
					<dict>
						<key>default value</key>
						<string></string>
						<key>name</key>
						<string>COMMAND_STRING</string>
						<key>required</key>
						<string>0</string>
						<key>type</key>
						<string>0</string>
					</dict>
					<key>4</key>
					<dict>
						<key>default value</key>
						<string>/bin/sh</string>
						<key>name</key>
						<string>shell</string>
						<key>required</key>
						<string>0</string>
						<key>type</key>
						<string>0</string>
						<key>value</key>
						<string>/bin/bash</string>
					</dict>
				</dict>
				<key>isViewVisible</key>
				<integer>1</integer>
				<key>location</key>
				<string>500.000000:305.000000</string>
				<key>nibPath</key>
				<string>/System/Library/Automator/Run Shell Script.action/Contents/Resources/Base.lproj/main.nib</string>
			</dict>
			<key>isViewVisible</key>
			<integer>1</integer>
		</dict>
	</array>
	<key>connectors</key>
	<dict/>
	<key>workflowMetaData</key>
	<dict>
		<key>applicationBundleIDsByPath</key>
		<dict/>
		<key>applicationPaths</key>
		<array/>
		<key>inputTypeIdentifier</key>
		<string>com.apple.Automator.fileSystemObject</string>
		<key>outputTypeIdentifier</key>
		<string>com.apple.Automator.nothing</string>
		<key>presentationMode</key>
		<integer>15</integer>
		<key>processesInput</key>
		<false/>
		<key>serviceApplicationBundleID</key>
		<string>com.apple.finder</string>
		<key>serviceApplicationPath</key>
		<string>/System/Library/CoreServices/Finder.app</string>
		<key>serviceInputTypeIdentifier</key>
		<string>com.apple.Automator.fileSystemObject</string>
		<key>serviceOutputTypeIdentifier</key>
		<string>com.apple.Automator.nothing</string>
		<key>serviceProcessesInput</key>
		<false/>
		<key>systemImageName</key>
		<string>NSTouchBarTransfer</string>
		<key>useAutomaticInputType</key>
		<false/>
		<key>workflowTypeIdentifier</key>
		<string>com.apple.Automator.servicesMenu</string>
	</dict>
</dict>
</plist>
EOF

chmod -R 755 "$TARGET_DIR"

# Refresh macOS Services cache
/System/Library/CoreServices/pbs -flush 2>/dev/null || true

echo "==> Quick Action installed successfully to $TARGET_DIR!"
echo "==> You can now right-click any file in Finder and select 'Quick Actions' -> 'Convert with UFC'."
