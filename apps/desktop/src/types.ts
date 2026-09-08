export type ParticipantKind = "human" | "ai";

export type Participant = {
  id: string;
  canonicalName: string;
  displayName: string;
  identityBadge: string;
  serviceLabel: string;
  kind: ParticipantKind;
  initials: string;
  accent: string;
  avatarUrl?: string;
  avatarPlacement?: AvatarPlacement;
};

export type AvatarPlacement = {
  scale: number;
  x: number;
  y: number;
};

export type AiAccessMode =
  | "providerDefault"
  | "chatOnly"
  | "workspaceRead"
  | "workspaceWrite";

export type ParticipantProfile = {
  participantId: string;
  displayName: string;
  avatar: ({ dataUrl: string } & AvatarPlacement) | null;
  aiInstructions: string;
  aiAccessMode: AiAccessMode;
  aiModel: string;
};

export type ChatMessage = {
  id: string;
  authorId: string;
  body: string;
  targetIds: string[];
  sentAt: string;
  isDemo?: boolean;
  provenance?: "codexOwnerProxy";
  artifactIds?: string[];
};

export type Room = {
  id: string;
  name: string;
  participantIds: string[];
  messages: ChatMessage[];
  updatedLabel: string;
};

export type ParticipantMap = Record<string, Participant>;

export type AiConnectionState =
  | "ready"
  | "installed"
  | "setupRequired"
  | "unsupported";

export type AiConnectionStatus = {
  participantId: string;
  state: AiConnectionState;
  label: string;
  detail: string;
};

export type AiConnectionMap = Record<string, AiConnectionStatus>;

export type RoomWorkspaceStatus = {
  roomId: string;
  mode: "chatOnly" | "workspace";
  folderName: string | null;
  available: boolean;
};

export type RoomBackupStatus = {
  directoryPath: string;
  isCustom: boolean;
  available: boolean;
};

export type RoomBackupPreview = {
  fileName: string;
  roomCount: number;
  createdAtUnixMs: number;
};

export type ConductorSendMode = "direct" | "conductor";

export type ImageGenerationComposition = "1:1" | "4:3" | "3:4" | "16:9" | "9:16";

export type ImageGenerationQuality = "auto" | "low" | "medium" | "high";

export type ImageGenerationPreferences = {
  composition: ImageGenerationComposition;
  quality: ImageGenerationQuality;
};

export type CodexTurnProgressPhase =
  | "preparing"
  | "thinking"
  | "reconnecting"
  | "workspace"
  | "tool"
  | "generatingImage"
  | "writingResponse";

export type CodexTurnProgress = {
  roomId: string;
  dispatchId: string;
  phase: CodexTurnProgressPhase;
  startedAt: number;
};

export type RoomConductorStatus = {
  roomId: string;
  conductorId: string | null;
  sendMode: ConductorSendMode;
};
