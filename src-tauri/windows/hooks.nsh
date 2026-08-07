!macro NSIS_HOOK_POSTINSTALL
  ; Delete Windows' cached user choice for the jms protocol.
  ; If an old Electron version was ever recorded as the default handler, this cache can keep overriding the new registration.
  DeleteRegKey HKCU "Software\Microsoft\Windows\Shell\Associations\UrlAssociations\jms\UserChoice"

  ; Explicitly write to HKCU\Software\Classes\jms.
  ; HKCU's protocol registration takes priority over HKLM, so it can override a machine-level registration an old version may have left behind.
  WriteRegStr HKCU "Software\Classes\jms" "URL Protocol" ""
  WriteRegStr HKCU "Software\Classes\jms" "" "URL:jms"
  WriteRegStr HKCU "Software\Classes\jms" "FriendlyTypeName" "JumpServer Client URL"

  ; The protocol icon shown in the system UI.
  WriteRegStr HKCU "Software\Classes\jms\DefaultIcon" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\",0"

  ; The command executed when opening a jms:// link. %1 is the raw link passed in by the browser or system.
  WriteRegStr HKCU "Software\Classes\jms\shell\open\command" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\""

  ; Notify the Windows shell that the protocol association has changed, to avoid the system continuing to use the old cache.
  System::Call 'shell32::SHChangeNotify(i, i, p, p) (0x08000000, 0x1000, 0, 0)'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; Only delete it if the current HKCU jms protocol actually points at this install directory, to avoid accidentally removing a registration another app has since taken over.
  ReadRegStr $R7 HKCU "Software\Classes\jms\shell\open\command" ""
  StrCmp $R7 "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\"" 0 +2
    DeleteRegKey HKCU "Software\Classes\jms"

  ; Notify the Windows shell that the protocol association has changed, to avoid it continuing to use the old cache after uninstall.
  System::Call 'shell32::SHChangeNotify(i, i, p, p) (0x08000000, 0x1000, 0, 0)'
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Tauri's updater invokes the old uninstaller with /UPDATE, which also runs this hook.
  ; Only delete data when the user is actually uninstalling, to avoid losing config, logs, and plugins during an upgrade.
  ${If} $UpdateMode <> 1
    ; installMode="both" may switch the shell context to all; user data always belongs to the current user.
    SetShellVarContext current

    ; Tauri's own app data/store/video directories.
    RMDir /r "$APPDATA\${BUNDLEID}"
    RMDir /r "$LOCALAPPDATA\${BUNDLEID}"

    ; Custom directory used for JumpServer config, logs, and user plugins.
    RMDir /r "$APPDATA\jumpserver-client"
    RMDir /r "$LOCALAPPDATA\jumpserver-client"

    ; Compatibility with the user data directory created by old versions using the product name.
    RMDir /r "$APPDATA\JumpServerClient"
    RMDir /r "$LOCALAPPDATA\JumpServerClient"
  ${EndIf}
!macroend
