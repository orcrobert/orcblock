; Hooks for the NSIS installer (tauri.windows.conf.json). The installer is
; per machine, so it already runs elevated and $INSTDIR is writable only by
; administrators: the helper it installs is the one it just copied there.

!macro NSIS_HOOK_POSTINSTALL
  nsExec::ExecToLog '"$INSTDIR\orcblockd.exe" install'
!macroend

; The helper refuses to uninstall while a block runs. Then the app goes and
; the helper stays until the block ends, so uninstalling never ends a block.
!macro NSIS_HOOK_PREUNINSTALL
  nsExec::ExecToLog '"$INSTDIR\orcblockd.exe" uninstall'
!macroend
