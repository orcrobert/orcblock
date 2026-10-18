; Hooks for the NSIS installer (tauri.windows.conf.json). The installer is
; per machine, so it already runs elevated and $INSTDIR is writable only by
; administrators: the helper it installs is the one it just copied there.

!macro NSIS_HOOK_POSTINSTALL
  nsExec::ExecToLog '"$INSTDIR\orcblockd.exe" install'
!macroend

; The helper refuses to uninstall while a block runs. The uninstaller then
; stops before removing anything, so the app stays to uninstall the helper
; once the block ends, and uninstalling never ends a block.
!macro NSIS_HOOK_PREUNINSTALL
  nsExec::ExecToLog '"$INSTDIR\orcblockd.exe" uninstall'
  Pop $0
  StrCmp $0 "0" +3
    MessageBox MB_OK|MB_ICONSTOP "OrcBlock cannot be removed while a block is running. Try again once it ends." /SD IDOK
    Abort
!macroend
