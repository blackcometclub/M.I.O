import {
  createContext,
  type ReactNode,
  useContext,
  useEffect,
  useMemo,
  useState,
} from "react";

export type UiLocale = "ja" | "en";

type UiPreferences = {
  chatFontScale: number;
  expandedWindowWidth: number;
  fontFamily: string;
  imageFreeMode: boolean;
  locale: UiLocale;
  participantListCollapsed: boolean;
  sidebarCollapsed: boolean;
  sidebarFontScale: number;
  sidebarWidth: number;
};

export const systemFontFamily = "__system__";
const storageKey = "moe-ui-preferences-v2";
const legacyStorageKey = "moe-ui-preferences-v1";
const defaultPreferences: UiPreferences = {
  chatFontScale: 1,
  expandedWindowWidth: 1440,
  fontFamily: systemFontFamily,
  imageFreeMode: false,
  locale: "ja",
  participantListCollapsed: false,
  sidebarCollapsed: false,
  sidebarFontScale: 1,
  sidebarWidth: 220,
};

const legacyFontFamilies: Record<string, string> = {
  system: systemFontFamily,
  "yu-gothic": "Yu Gothic UI",
  meiryo: "Meiryo",
  "biz-udp-gothic": "BIZ UDPGothic",
};
const systemFontStack = 'Inter, "Yu Gothic UI", "Hiragino Kaku Gothic ProN", system-ui, sans-serif';

function clamp(value: number, minimum: number, maximum: number) {
  return Math.min(maximum, Math.max(minimum, value));
}

function normalizeFontFamily(value: unknown) {
  if (value === systemFontFamily) return systemFontFamily;
  if (typeof value !== "string" || value.length === 0 || value.length > 256 || /[\u0000-\u001f\u007f]/u.test(value)) {
    return systemFontFamily;
  }
  return value;
}

function fontStack(fontFamily: string) {
  return fontFamily === systemFontFamily ? systemFontStack : `${JSON.stringify(fontFamily)}, ${systemFontStack}`;
}

const ja = {
  roomConductor: "指揮者",
  roomConductorHelp: "このルームの依頼をまとめるAIを選びます",
  roomConductorSelection: "指揮者AI",
  noConductor: "指揮者なし",
  conductorModeChangeHelp: "送信欄で Direct / Conductor をいつでも切り替えられます",
  conductorBadge: "指揮者",
  sendMode: "送信モード",
  appLabel: "M.I.O. トークルーム",
  rooms: "トークルーム",
  roomList: "ルーム一覧",
  newRoom: "新しいルーム",
  aiCount: "AI {count}人",
  noConversation: "まだ会話なし",
  now: "いま",
  roomSettings: "ルーム設定",
  appearance: "背景を変更",
  hideDecorativeImages: "画像なし表示にする",
  showDecorativeImages: "画像を表示する",
  hideRoomList: "ルーム一覧を閉じる",
  showRoomList: "ルーム一覧を開く",
  preferences: "環境設定",
  close: "閉じる",
  minimize: "最小化",
  maximize: "最大化または元に戻す",
  participants: "参加AI",
  chooseRecipients: "クリックで今回の宛先を選択",
  collapseParticipants: "参加AI一覧を畳む",
  expandParticipants: "参加AI一覧を開く",
  checking: "確認中",
  checkingDetail: "接続状態を確認しています。",
  addAi: "AIを追加",
  addToRoom: "ルームに追加",
  addStatusNote: "追加後も接続状態は個別に表示されます",
  currentlyUnsupported: "現在未対応",
  allAiAdded: "追加できるAIはすべて参加中です。",
  conversation: "会話",
  messageActions: "{name}の発言操作",
  copyMessage: "メッセージをコピー",
  copyMessageFailed: "コピーできませんでした",
  viaCodex: "via Codex",
  roomGeneratedImage: "AIが生成した画像",
  roomImageLoading: "画像を読み込んでいます…",
  roomImageLoadFailed: "画像を読み込めませんでした。",
  roomImageOpenOriginal: "クリックして原寸表示",
  roomImageDownload: "ダウンロード",
  roomImageSaveWorkspace: "編集フォルダへ保存",
  roomImageSaving: "保存中…",
  roomImageSaved: "保存しました",
  roomImageSaveTitle: "画像を編集フォルダへ保存",
  roomImageSaveDescription: "ファイル名を確認してください。設定済みの編集フォルダへ、新しいファイルとして保存します。",
  roomImageFileName: "ファイル名",
  roomImageFileNameInvalid: "同じ画像形式の拡張子を付け、Windowsで使えるファイル名を入力してください。",
  roomImageSaveConfirm: "保存",
  roomImageSaveFailed: "保存できませんでした。ファイルが既にあるか、編集フォルダを確認してください。",
  quietRoom: "まだ静かなルームです",
  firstMessage: "宛先を選んで、最初のメッセージを送ってみましょう。",
  thinking: "{name} が考え中",
  codexProgress: "{name} · {phase}",
  codexProgressElapsed: " · {seconds}秒",
  codexProgressPreparing: "準備中",
  codexProgressThinking: "考え中",
  codexProgressReconnecting: "再接続中",
  codexProgressWorkspace: "ファイルを操作中",
  codexProgressTool: "ツールを実行中",
  codexProgressGeneratingImage: "画像を生成中",
  codexProgressWritingResponse: "返答を整理中",
  removeRecipient: "{name}を宛先から外す",
  chooseRecipientFirst: "上の参加AIから宛先を選んでください",
  message: "メッセージ",
  messagePlaceholder: "M.I.O.のみんなに相談してみる…",
  saving: "保存中",
  waiting: "応答待ち",
  send: "送信",
  stop: "停止",
  stopping: "停止中",
  imageGenerationSettings: "画像生成の設定",
  imageGenerationComposition: "アスペクト比",
  imageGenerationQuality: "品質の希望",
  imageGenerationSettingsHelp: "Codex内蔵の画像生成へ希望として渡します。アスペクト比と品質は生成結果により異なる場合があります。正確な画素数指定には対応していません。M.I.O.が受け取れる画像は16 MiBまでです。",
  imageAspectSquare: "正方形（1:1）",
  imageAspectRatio: "アスペクト比 {ratio}",
  imageQualityAuto: "自動",
  imageQualityLow: "低（下書き向け）",
  imageQualityMedium: "中",
  imageQualityHigh: "高",
  commandConfirmationTitle: "この操作を許可しますか？",
  commandConfirmationSubtitle: "M.I.O.本体が、実行直前に確認しています。表示された操作と対象を確認してください。",
  commandConfirmationAction: "操作",
  commandConfirmationReason: "確認が必要な理由",
  commandConfirmationTarget: "対象",
  commandConfirmationAllowOnce: "今回だけ許可",
  commandConfirmationAllowRoomSession: "このRoomを開いている間は許可",
  commandConfirmationDeny: "拒否",
  commandConfirmationResolving: "確認中…",
  commandConfirmationFailed: "回答を確定できませんでした。操作は許可されていません。",
  commandConfirmationMore: "このあとに確認があと{count}件あります。",
  commandConfirmationWaitingHint: "安全確認への回答を待っています…",
  commandActionGitPush: "Gitの変更を外部へpush",
  commandActionPackageInstall: "npm packageを取得・導入",
  commandActionCargoFetch: "Cargo dependencyを取得",
  commandActionUnregisteredTool: "未登録toolを実行",
  commandActionCredentialUse: "credentialを使用",
  commandActionAdministratorOperation: "管理者権限を使用",
  commandActionDestructiveOperation: "回復が難しい変更を実行",
  commandReasonExternalMutation: "端末の外にある状態を変更します",
  commandReasonNetworkDownload: "networkからfileを取得します",
  commandReasonUnregisteredTool: "検証済み一覧にないtoolです",
  commandReasonCredentialUse: "秘密情報またはaccount権限を使います",
  commandReasonPrivilegeExpansion: "通常より強いWindows権限が必要です",
  commandReasonDestructiveChange: "削除や上書きなど影響の大きい変更です",
  dismissSafetyWarning: "確認済みとして閉じる",
  appearanceTitle: "M.I.O.本体をきせかえ",
  appearanceSubtitle: "黄色い部分だけを変更します",
  backgroundColor: "背景色",
  colorPresets: "背景色プリセット",
  bodyBackground: "本体の背景画像",
  chooseBackground: "背景を選ぶ",
  clearBackground: "背景画像を外す",
  artwork: "M.I.O.全体の飾り絵",
  artworkHelp: "Chatツリーを含む黄色い部分全体へ、位置と倍率を指定します",
  chooseArtwork: "飾り絵を選ぶ",
  editPlacement: "配置を調整",
  clearArtwork: "飾り絵を外す",
  artworkNote: "M.I.O.のウインドウ全体に合わせた画像がおすすめです。",
  appearanceLoading: "保存済みのきせかえを読み込み中…",
  appearanceSaving: "この端末へきせかえを保存中…",
  appearanceSaved: "きせかえはこの端末へ自動保存されます。",
  appearanceSaveFailed: "きせかえを保存できませんでした。画像サイズをご確認ください。",
  preferencesTitle: "環境設定",
  preferencesSubtitle: "文字と表示言語を変更します",
  language: "言語",
  japanese: "日本語",
  english: "English",
  fontFamily: "フォント",
  fontSize: "チャット文字サイズ",
  fontSizeValue: "{value}%",
  systemFont: "システム標準",
  installedFontsLoading: "インストール済みフォントを読み込み中…",
  noInstalledFonts: "フォント一覧を取得できませんでした。システム標準を使用できます。",
  sidebarTextSmaller: "ツリーの文字を小さくする",
  sidebarTextLarger: "ツリーの文字を大きくする",
  resizeRoomTree: "トークルームツリーの幅を変更",
  preferenceSaved: "変更はこの端末に自動保存されます。",
  roomSettingsSubtitle: "名前と参加AIを管理します",
  participantProfiles: "参加者プロフィール",
  profileDeviceNote: "この端末で共通",
  editProfile: "編集",
  aiMembership: "参加AIの管理",
  aiContinuity: "AIの会話継続",
  aiContinuityHelp: "AI側の継続だけを解除します",
  roomHistoryKept: "Roomの会話履歴は残ります",
  resetContinuity: "継続をリセット",
  profileTitle: "参加者プロフィールを編集",
  profileSubtitle: "表示名と丸いアイコンを、この端末用に設定します",
  identityLocked: "正式な身元",
  displayName: "表示名",
  aiInstructions: "AIの基本設定",
  aiInstructionsHelp: "このAIの話し方・役割・あなたの呼び方を指定します。次の発言から反映されます。",
  aiInstructionsPlaceholder: "例：いつもノリノリで、親しみやすく返事をする。太郎さんと呼ぶ。",
  aiModel: "使用モデル",
  aiModelHelp: "通常はProviderの既定を使います。M.I.O.で確認済みの候補だけ固定できます。",
  providerDefaultModel: "Providerの既定",
  modelContractNote: "利用可否・料金・保持は各Providerの契約に従います。利用不能時に別モデルへ自動変更しません。",
  aiPermissions: "AIのアクセス権限",
  aiPermissionsHelp: "この端末でM.I.O.から呼ぶ時の上限です。実装済みの安全な権限だけ選べます。",
  permissionChatOnly: "会話のみ",
  permissionChatOnlyDetail: "Roomの会話だけを送り、ローカルファイルやコマンドを使いません。",
  permissionWorkspaceRead: "選択フォルダーを読む",
  permissionWorkspaceReadDetail: "CodexがM.I.O.経由で、名前を指定したUTF-8テキストファイルだけを読みます。コマンドとWebは使いません。",
  permissionGrokWorkspaceReadDetail: "GrokにはM.I.O.が作った追跡済みGit差分と状態だけを渡します。未追跡ファイル本文・編集・コマンド・Webは使いません。",
  permissionWorkspaceWrite: "選択フォルダーを読取り・編集",
  permissionWorkspaceWriteDetail: "読取りに加え、UTF-8テキストファイルの新規作成と安全な置換を許可します。限定コマンドの一部は実行前に確認します。削除・名前変更・Webは使いません。",
  permissionNotSupported: "このAIは未対応",
  permissionFiles: "ファイル",
  permissionReadOnly: "選択フォルダー内を読取り",
  permissionReadWrite: "選択フォルダー内を読取り・編集",
  permissionCommands: "コマンド",
  permissionWeb: "Web・ネットワーク",
  permissionOff: "禁止",
  permissionBoundedCommands: "限定操作・一部は確認",
  permissionLocalOnly: "選択フォルダー内のみ",
  permissionNeedsWorkspace: "この権限を使うには、ルーム設定で作業フォルダーを選んでください。未選択時は会話のみになります。",
  avatarImage: "アイコン画像",
  avatarPositionHelp: "画像をドラッグして位置を調整できます。PNG・JPEG・WebPに対応。",
  chooseAvatar: "画像を選ぶ",
  changeAvatar: "画像を変更",
  removeAvatar: "画像を外す",
  avatarZoom: "拡大率",
  profilePreview: "見え方",
  invalidAvatarImage: "この画像は読み込めません。PNG・JPEG・WebPを選んでください。",
  avatarImageTooLarge: "画像が大きすぎます。5MB以下の画像を選んでください。",
  profileSaveFailed: "保存できませんでした。表示名と画像をご確認ください。",
  profileSaving: "保存中…",
  saveProfile: "プロフィールを保存",
  cancel: "キャンセル",
  roomName: "ルーム名",
  save: "保存",
  backupTitle: "全ルームのバックアップ",
  backupHelp: "現在表示している保存先へ安全なJSONを保存します。保存先を変えても既存ファイルは移動しません。",
  backupDefaultLocation: "既定の保存先",
  backupCustomLocation: "選択した保存先",
  backupLocationLoading: "保存先を確認中…",
  backupLocationMissing: "このフォルダーは見つかりません。別の場所へは保存しません。",
  backupChooseLocation: "保存先を選ぶ",
  backupOpenLocation: "フォルダーを開く",
  backupUseDefault: "既定に戻す",
  backup: "バックアップ",
  restore: "復元内容を確認…",
  reallyRestore: "確認した内容を復元",
  restorePreviewTitle: "復元するバックアップ",
  restorePreviewRooms: "{count}室を復元します",
  restorePreviewWarning: "現在のRoomと会話履歴を、このバックアップの内容へ置き換えます。",
  codexMode: "Codex作業モード",
  workspaceHelp: "Codexに使わせるフォルダーをRoom単位で選びます。実際の権限はCodexの参加者プロフィールで設定します。",
  workspaceActive: "{name} を選択中です。実際の権限はCodexの参加者プロフィールで設定します。",
  workspaceMissing: "{name} が見つかりません。",
  chatOnly: "現在は会話のみ。ローカルファイルを見ません。",
  changeFolder: "フォルダー変更",
  chooseFolder: "フォルダーを選ぶ",
  returnChatOnly: "会話のみに戻す",
  keepOneAi: "最低1人は残します",
  historyKeepsAi: "履歴に使われているため保持",
  cannotRemoveAi: "履歴または最低人数を守るため外せません",
  removeFromRoom: "ルームから外す",
  remove: "外す",
  deleteRoom: "このルームを削除",
  deleteRoomHelp: "会話履歴も一緒に削除され、元に戻せません。",
  roomActions: "{name}の操作",
  openRoom: "ルームを開く",
  renameRoom: "名前を変更…",
  deleteRoomConfirm: "「{name}」と会話履歴を削除しますか？",
  deleteRoomFailed: "削除できませんでした。ルームは変更されていません。",
  delete: "削除…",
  reallyDelete: "本当に削除",
  protectedRoom: "標準ルームは削除から保護されています。",
  coreOffline: "Room offline",
  coreConnecting: "Room connecting",
  coreReady: "Core + Room ready",
  coreReadyToolsChecking: "Core + Room ready · 開発環境を確認中…",
  previewReady: "Preview ready",
  awaitingHint: "接続済みAIからの応答を待っています…",
  anotherRoomAwaitingHint: "「{name}」でAIが応答中です · Roomを戻すと進捗確認と停止ができます",
  roomAiWorking: "AI応答中",
  workspaceMissingHint: "Codex作業フォルダーが見つかりません · ルーム設定をご確認ください",
  unconnectedHint: "未接続AI宛はRoomへ保存のみ · 現在は返信がありません",
  workspaceHint: "Codex作業モード · {name}内を読取り・編集できます",
  sendHint: "Enter で送信 · Shift + Enter で改行 · Room保存後、接続済みAIへ配送します",
  roomSettingsLoadingHint: "このRoomの設定を確認中です…",
  roomUnavailableHint: "Rust Roomが利用できるまで送信できません",
  demoHint: "Enter で送信 · Shift + Enter で改行 · 現在はダミー応答です",
  selectedFolder: "選択フォルダー",
} as const;

type UiTextKey = keyof typeof ja;
const en: Record<UiTextKey, string> = {
  roomConductor: "Conductor",
  roomConductorHelp: "Choose the AI that coordinates requests in this room.",
  roomConductorSelection: "Conductor AI",
  noConductor: "No conductor",
  conductorModeChangeHelp: "You can switch between Direct and Conductor in the composer at any time.",
  conductorBadge: "Conductor",
  sendMode: "Send mode",
  appLabel: "M.I.O. Talk Rooms", rooms: "Talk rooms", roomList: "Room list", newRoom: "New room",
  aiCount: "{count} AI", noConversation: "No messages yet", now: "now", roomSettings: "Room settings",
  appearance: "Change appearance", hideDecorativeImages: "Switch to image-free view", showDecorativeImages: "Show decorative images",
  hideRoomList: "Close room list", showRoomList: "Open room list", preferences: "Preferences", close: "Close", minimize: "Minimize",
  maximize: "Maximize or restore", participants: "Participating AI", chooseRecipients: "Click to choose recipients for this message",
  collapseParticipants: "Collapse participating AI", expandParticipants: "Expand participating AI",
  checking: "Checking", checkingDetail: "Checking connection status.", addAi: "Add AI", addToRoom: "Add to room",
  addStatusNote: "Connection status remains visible after adding", currentlyUnsupported: "Currently unsupported", allAiAdded: "All available AI are already participating.",
  conversation: "Conversation", messageActions: "Actions for {name}'s message", copyMessage: "Copy message", copyMessageFailed: "Could not copy",
  viaCodex: "via Codex", roomGeneratedImage: "AI-generated image", roomImageLoading: "Loading image…",
  roomImageLoadFailed: "The image could not be loaded.", roomImageOpenOriginal: "Open at original size", roomImageDownload: "Download",
  roomImageSaveWorkspace: "Save to editing folder", roomImageSaving: "Saving…", roomImageSaved: "Saved",
  roomImageSaveTitle: "Save image to editing folder", roomImageSaveDescription: "Confirm the file name. M.I.O. will create a new file in the editing folder selected for this Room.",
  roomImageFileName: "File name", roomImageFileNameInvalid: "Enter a Windows-compatible file name using the image's current extension.", roomImageSaveConfirm: "Save",
  roomImageSaveFailed: "The image could not be saved. Check whether the file already exists and whether an editing folder is selected.",
  quietRoom: "This room is still quiet", firstMessage: "Choose recipients and send the first message.",
  thinking: "{name} is thinking", removeRecipient: "Remove {name} from recipients", chooseRecipientFirst: "Choose recipients from the participating AI above",
  codexProgress: "{name} · {phase}", codexProgressElapsed: " · {seconds}s", codexProgressPreparing: "Preparing", codexProgressThinking: "Thinking", codexProgressReconnecting: "Reconnecting",
  codexProgressWorkspace: "Working with files", codexProgressTool: "Running tools", codexProgressGeneratingImage: "Generating image",
  codexProgressWritingResponse: "Preparing response",
  message: "Message", messagePlaceholder: "Ask everyone in M.I.O.…", saving: "Saving", waiting: "Waiting", send: "Send", stop: "Stop", stopping: "Stopping",
  imageGenerationSettings: "Image generation settings", imageGenerationComposition: "Aspect ratio", imageGenerationQuality: "Quality preference",
  imageGenerationSettingsHelp: "M.I.O. passes these as preferences to Codex's built-in image generator. The aspect ratio and quality may differ in the result. Exact pixel dimensions are not supported. M.I.O. accepts images up to 16 MiB.",
  imageAspectSquare: "Square (1:1)", imageAspectRatio: "Aspect ratio {ratio}",
  imageQualityAuto: "Auto", imageQualityLow: "Low · draft", imageQualityMedium: "Medium", imageQualityHigh: "High",
  commandConfirmationTitle: "Allow this operation?",
  commandConfirmationSubtitle: "M.I.O. is asking immediately before execution. Check the operation and target shown below.",
  commandConfirmationAction: "Operation", commandConfirmationReason: "Why confirmation is required", commandConfirmationTarget: "Target",
  commandConfirmationAllowOnce: "Allow once", commandConfirmationAllowRoomSession: "Allow while this Room is open", commandConfirmationDeny: "Deny",
  commandConfirmationResolving: "Confirming…", commandConfirmationFailed: "The decision could not be confirmed. The operation has not been allowed.",
  commandConfirmationMore: "{count} more confirmation request(s) will follow.", commandConfirmationWaitingHint: "Waiting for your safety decision…",
  commandActionGitPush: "Push Git changes externally", commandActionPackageInstall: "Download and install npm packages",
  commandActionCargoFetch: "Download Cargo dependencies", commandActionUnregisteredTool: "Run an unregistered tool",
  commandActionCredentialUse: "Use credentials", commandActionAdministratorOperation: "Use administrator privileges",
  commandActionDestructiveOperation: "Perform a difficult-to-recover change",
  commandReasonExternalMutation: "Changes state outside this device", commandReasonNetworkDownload: "Downloads files from the network",
  commandReasonUnregisteredTool: "The tool is not in the verified list", commandReasonCredentialUse: "Uses secrets or account authority",
  commandReasonPrivilegeExpansion: "Requires stronger Windows privileges", commandReasonDestructiveChange: "Deletes or overwrites data with significant impact",
  dismissSafetyWarning: "Acknowledge and close",
  appearanceTitle: "Dress up M.I.O.", appearanceSubtitle: "Changes only the colored shell", backgroundColor: "Background color",
  colorPresets: "Background color presets", bodyBackground: "Shell background image", chooseBackground: "Choose background",
  clearBackground: "Remove background image", artwork: "M.I.O. full-shell artwork", artworkHelp: "Position and scale artwork across the colored shell, including the Chat tree",
  chooseArtwork: "Choose artwork", editPlacement: "Adjust placement", clearArtwork: "Remove artwork",
  artworkNote: "An image matching the whole M.I.O. window works best.", preferencesTitle: "Preferences",
  appearanceLoading: "Loading saved appearance…", appearanceSaving: "Saving appearance to this device…",
  appearanceSaved: "Appearance is saved automatically on this device.",
  appearanceSaveFailed: "Appearance could not be saved. Check the image size.",
  preferencesSubtitle: "Change text and interface language", language: "Language", japanese: "日本語", english: "English",
  fontFamily: "Font", fontSize: "Chat text size", fontSizeValue: "{value}%", preferenceSaved: "Changes are saved automatically on this device.",
  systemFont: "System default", installedFontsLoading: "Loading installed fonts…",
  noInstalledFonts: "The installed font list could not be loaded. System default remains available.",
  sidebarTextSmaller: "Make room-tree text smaller", sidebarTextLarger: "Make room-tree text larger",
  resizeRoomTree: "Resize the talk-room tree",
  roomSettingsSubtitle: "Manage the name and participating AI", roomName: "Room name", save: "Save",
  participantProfiles: "Participant profiles", profileDeviceNote: "Shared on this device", editProfile: "Edit",
  aiMembership: "Room AI membership", profileTitle: "Edit participant profile",
  aiContinuity: "AI conversation continuity", aiContinuityHelp: "Clears only the AI-side continuation",
  roomHistoryKept: "Room conversation history is kept", resetContinuity: "Reset continuity",
  profileSubtitle: "Set a display name and circular avatar for this device", identityLocked: "Verified identity",
  displayName: "Display name", avatarImage: "Avatar image",
  aiInstructions: "AI defaults", aiInstructionsHelp: "Set this AI's tone, role, and how it addresses you. Applies from the next message.",
  aiInstructionsPlaceholder: "Example: Be energetic and friendly. Address me as John.",
  aiModel: "Model", aiModelHelp: "Use the provider default normally, or pin one model verified by M.I.O.",
  providerDefaultModel: "Provider default",
  modelContractNote: "Availability, billing, and retention follow the provider contract. M.I.O. does not silently switch models.",
  aiPermissions: "AI access permissions", aiPermissionsHelp: "This is the maximum access M.I.O. grants on this device. Only verified permissions can be selected.",
  permissionChatOnly: "Conversation only", permissionChatOnlyDetail: "Sends only the Room conversation. Local files and commands are unavailable.",
  permissionWorkspaceRead: "Read selected folder", permissionWorkspaceReadDetail: "Codex reads only named UTF-8 text files through M.I.O. Commands and web access remain blocked.",
  permissionGrokWorkspaceReadDetail: "Grok receives only the tracked Git patch and status prepared by M.I.O. Untracked file bodies, edits, commands, and web access remain blocked.",
  permissionWorkspaceWrite: "Read and edit selected folder", permissionWorkspaceWriteDetail: "Adds UTF-8 text-file creation and safe replacement. Some bounded commands require confirmation before execution. Delete, rename, and web access remain blocked.",
  permissionNotSupported: "Not supported for this AI", permissionFiles: "Files", permissionReadOnly: "Read inside selected folder", permissionReadWrite: "Read and edit inside selected folder", permissionCommands: "Commands", permissionWeb: "Web and network",
  permissionOff: "Blocked", permissionBoundedCommands: "Bounded operations; some require confirmation", permissionLocalOnly: "Selected folder only",
  permissionNeedsWorkspace: "Choose a workspace folder in Room settings to use this permission. Without one, the AI remains conversation-only.",
  avatarPositionHelp: "Drag the image to reposition it. PNG, JPEG, and WebP are supported.",
  chooseAvatar: "Choose image", changeAvatar: "Change image", removeAvatar: "Remove image", avatarZoom: "Zoom",
  profilePreview: "Preview", invalidAvatarImage: "This image could not be read. Choose a PNG, JPEG, or WebP file.",
  avatarImageTooLarge: "The image is too large. Choose an image up to 5 MB.",
  profileSaveFailed: "The profile could not be saved. Check the display name and image.",
  profileSaving: "Saving…", saveProfile: "Save profile", cancel: "Cancel",
  backupTitle: "Back up all rooms", backupHelp: "Saves safe JSON to the displayed directory. Changing the directory does not move existing files.",
  backupDefaultLocation: "Default directory", backupCustomLocation: "Selected directory", backupLocationLoading: "Checking directory…",
  backupLocationMissing: "This folder is missing. M.I.O. will not save somewhere else.", backupChooseLocation: "Choose directory",
  backupOpenLocation: "Open folder", backupUseDefault: "Use default", backup: "Back up",
  restore: "Review restore…", reallyRestore: "Restore reviewed backup", restorePreviewTitle: "Backup to restore",
  restorePreviewRooms: "Restore {count} rooms", restorePreviewWarning: "Current Rooms and conversation history will be replaced with this backup.", codexMode: "Codex workspace mode",
  workspaceHelp: "Choose one Room-scoped folder for Codex. Set the actual access level in the Codex participant profile.",
  workspaceActive: "{name} is selected. Set the actual access level in the Codex participant profile.", workspaceMissing: "{name} could not be found.",
  chatOnly: "Chat only. Local files are not accessed.", changeFolder: "Change folder", chooseFolder: "Choose folder",
  returnChatOnly: "Return to chat only", keepOneAi: "Keep at least one", historyKeepsAi: "Kept because it appears in history",
  cannotRemoveAi: "Cannot remove because of history or the minimum participant count", removeFromRoom: "Remove from room", remove: "Remove",
  deleteRoom: "Delete this room", deleteRoomHelp: "Conversation history will also be deleted and cannot be restored.",
  roomActions: "Actions for {name}", openRoom: "Open room", renameRoom: "Rename…", deleteRoomConfirm: "Delete {name} and its conversation history?", deleteRoomFailed: "The Room could not be deleted and was not changed.",
  delete: "Delete…", reallyDelete: "Confirm delete", protectedRoom: "Default rooms are protected from deletion.",
  coreOffline: "Room offline", coreConnecting: "Room connecting", coreReady: "Core + Room ready",
  coreReadyToolsChecking: "Core + Room ready · Checking development tools…", previewReady: "Preview ready",
  awaitingHint: "Waiting for connected AI…", anotherRoomAwaitingHint: "AI is responding in {name} · Return there to view progress or stop", roomAiWorking: "AI responding",
  workspaceMissingHint: "Codex workspace was not found · Check Room settings",
  unconnectedHint: "Saved to the Room only for disconnected AI · No reply is available", workspaceHint: "Codex workspace · Can read and edit inside {name}",
  sendHint: "Enter to send · Shift + Enter for a new line · Delivered after Room save", roomUnavailableHint: "Sending is disabled until Rust Room is available",
  roomSettingsLoadingHint: "Checking this Room's settings…",
  demoHint: "Enter to send · Shift + Enter for a new line · Demo replies are active", selectedFolder: "selected folder",
};

function loadPreferences(): UiPreferences {
  try {
    type StoredPreferences = Partial<UiPreferences> & { font?: string; fontScale?: number };
    const value = JSON.parse(localStorage.getItem(storageKey) ?? localStorage.getItem(legacyStorageKey) ?? "null") as StoredPreferences | null;
    const legacyFont = typeof value?.font === "string" ? legacyFontFamilies[value.font] : undefined;
    const fontFamily = normalizeFontFamily(value?.fontFamily ?? legacyFont);
    const locale = value?.locale === "en" || value?.locale === "ja" ? value.locale : defaultPreferences.locale;
    const rawChatScale = typeof value?.chatFontScale === "number"
      ? value.chatFontScale
      : typeof value?.fontScale === "number" ? value.fontScale : defaultPreferences.chatFontScale;
    const rawSidebarScale = typeof value?.sidebarFontScale === "number" ? value.sidebarFontScale : defaultPreferences.sidebarFontScale;
    const rawSidebarWidth = typeof value?.sidebarWidth === "number" ? value.sidebarWidth : defaultPreferences.sidebarWidth;
    const rawExpandedWindowWidth = typeof value?.expandedWindowWidth === "number"
      ? value.expandedWindowWidth
      : defaultPreferences.expandedWindowWidth;
    return {
      chatFontScale: clamp(rawChatScale, 0.8, 1.5),
      expandedWindowWidth: clamp(rawExpandedWindowWidth, 1080, 3840),
      fontFamily,
      imageFreeMode: typeof value?.imageFreeMode === "boolean"
        ? value.imageFreeMode
        : defaultPreferences.imageFreeMode,
      locale,
      participantListCollapsed: typeof value?.participantListCollapsed === "boolean"
        ? value.participantListCollapsed
        : defaultPreferences.participantListCollapsed,
      sidebarCollapsed: typeof value?.sidebarCollapsed === "boolean"
        ? value.sidebarCollapsed
        : defaultPreferences.sidebarCollapsed,
      sidebarFontScale: clamp(rawSidebarScale, 0.8, 1.3),
      sidebarWidth: clamp(rawSidebarWidth, 180, 420),
    };
  } catch {
    return defaultPreferences;
  }
}

type UiPreferencesContextValue = UiPreferences & {
  setChatFontScale: (scale: number) => void;
  setExpandedWindowWidth: (width: number) => void;
  setFontFamily: (fontFamily: string) => void;
  setImageFreeMode: (enabled: boolean) => void;
  setLocale: (locale: UiLocale) => void;
  setParticipantListCollapsed: (collapsed: boolean) => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  setSidebarFontScale: (scale: number) => void;
  setSidebarWidth: (width: number) => void;
  t: (key: UiTextKey, values?: Record<string, string | number>) => string;
};

const UiPreferencesContext = createContext<UiPreferencesContextValue | null>(null);

export function UiPreferencesProvider({ children }: { children: ReactNode }) {
  const [preferences, setPreferences] = useState(loadPreferences);

  useEffect(() => {
    try {
      localStorage.setItem(storageKey, JSON.stringify(preferences));
    } catch {
      // Keep the preferences usable for this session when storage is unavailable.
    }
    document.documentElement.lang = preferences.locale;
    document.documentElement.style.setProperty("--moe-chat-font-scale", String(preferences.chatFontScale));
    document.documentElement.style.setProperty("--moe-font-family", fontStack(preferences.fontFamily));
    document.documentElement.style.setProperty("--moe-sidebar-font-scale", String(preferences.sidebarFontScale));
    document.documentElement.style.setProperty("--moe-sidebar-width", `${preferences.sidebarWidth}px`);
  }, [preferences]);

  const value = useMemo<UiPreferencesContextValue>(() => ({
    ...preferences,
    setChatFontScale: (chatFontScale) => setPreferences((current) => ({ ...current, chatFontScale: clamp(chatFontScale, 0.8, 1.5) })),
    setExpandedWindowWidth: (expandedWindowWidth) => setPreferences((current) => ({ ...current, expandedWindowWidth: clamp(expandedWindowWidth, 1080, 3840) })),
    setFontFamily: (fontFamily) => setPreferences((current) => ({ ...current, fontFamily: normalizeFontFamily(fontFamily) })),
    setImageFreeMode: (imageFreeMode) => setPreferences((current) => ({ ...current, imageFreeMode })),
    setLocale: (locale) => setPreferences((current) => ({ ...current, locale })),
    setParticipantListCollapsed: (participantListCollapsed) => setPreferences((current) => ({ ...current, participantListCollapsed })),
    setSidebarCollapsed: (sidebarCollapsed) => setPreferences((current) => ({ ...current, sidebarCollapsed })),
    setSidebarFontScale: (sidebarFontScale) => setPreferences((current) => ({ ...current, sidebarFontScale: clamp(sidebarFontScale, 0.8, 1.3) })),
    setSidebarWidth: (sidebarWidth) => setPreferences((current) => ({ ...current, sidebarWidth: clamp(sidebarWidth, 180, 420) })),
    t: (key, values = {}) => Object.entries(values).reduce(
      (text, [name, replacement]) => text.replaceAll(`{${name}}`, String(replacement)),
      (preferences.locale === "ja" ? ja[key] : en[key]) as string,
    ),
  }), [preferences]);

  return <UiPreferencesContext.Provider value={value}>{children}</UiPreferencesContext.Provider>;
}

export function useUiPreferences() {
  const value = useContext(UiPreferencesContext);
  if (!value) throw new Error("UiPreferencesProvider is missing");
  return value;
}
