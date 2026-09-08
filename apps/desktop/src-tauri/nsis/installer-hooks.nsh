; Tauri keeps the installation details page open by default so diagnostic
; output can be reviewed. M.I.O. advances to the finish page automatically
; after every installation step has completed successfully.
!macro NSIS_HOOK_POSTINSTALL
  SetAutoClose true
!macroend
