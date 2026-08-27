import type { ArtworkSource } from "../artwork";
import { ArtworkCanvas } from "./ArtworkCanvas";

type ArtworkStageProps = {
  artwork: ArtworkSource | null;
};

export function ArtworkStage({ artwork }: ArtworkStageProps) {
  if (!artwork) {
    return null;
  }

  return (
    <ArtworkCanvas
      ariaLabel="M.I.O.全体の飾り絵"
      className="artwork-stage"
      dataUrl={artwork.dataUrl}
      imageAlt="設定したM.I.O.全体の飾り絵"
      placement={artwork.placement}
    />
  );
}
