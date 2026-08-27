import type { CSSProperties } from "react";

import type { Participant } from "../types";

type AvatarProps = {
  participant: Participant;
  size?: "small" | "medium" | "large";
};

type AvatarStyle = CSSProperties & {
  "--avatar-accent": string;
};

export function Avatar({ participant, size = "medium" }: AvatarProps) {
  const avatarStyle: AvatarStyle = {
    "--avatar-accent": participant.accent,
  };

  return (
    <span
      aria-hidden="true"
      className={`avatar-wrap avatar-${size}`}
      style={avatarStyle}
    >
      <span className="avatar">
        {participant.avatarUrl ? (
          <img
            alt=""
            src={participant.avatarUrl}
            style={participant.avatarPlacement ? {
              objectPosition: `${50 + participant.avatarPlacement.x * 50}% ${50 + participant.avatarPlacement.y * 50}%`,
              transform: `translate(${-participant.avatarPlacement.x * (participant.avatarPlacement.scale - 1) * 50 / participant.avatarPlacement.scale}%, ${-participant.avatarPlacement.y * (participant.avatarPlacement.scale - 1) * 50 / participant.avatarPlacement.scale}%) scale(${participant.avatarPlacement.scale})`,
            } : undefined}
          />
        ) : (
          <span>{participant.initials}</span>
        )}
      </span>
      <span className="avatar-identity-badge">{participant.identityBadge}</span>
    </span>
  );
}
