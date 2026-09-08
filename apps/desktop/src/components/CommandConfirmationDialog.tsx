import { type KeyboardEvent, useEffect, useRef } from "react";

import type {
  CommandConfirmationAction,
  CommandConfirmationDecision,
  CommandConfirmationReason,
  CommandConfirmationRequest,
} from "../commandConfirmationBridge";
import { useUiPreferences } from "../uiPreferences";

type CommandConfirmationDialogProps = {
  error: boolean;
  isResolving: boolean;
  onDecision: (decision: CommandConfirmationDecision) => Promise<boolean>;
  remainingCount: number;
  request: CommandConfirmationRequest;
};

export function CommandConfirmationDialog({
  error,
  isResolving,
  onDecision,
  remainingCount,
  request,
}: CommandConfirmationDialogProps) {
  const { t } = useUiPreferences();
  const dialogRef = useRef<HTMLElement>(null);
  const denyButtonRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    const previousFocus = document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null;
    denyButtonRef.current?.focus();
    return () => previousFocus?.focus();
  }, [request.requestId]);

  useEffect(() => {
    const handleEscape = (event: globalThis.KeyboardEvent) => {
      if (event.key !== "Escape" || isResolving) return;
      event.preventDefault();
      void onDecision("dismissed");
    };
    document.addEventListener("keydown", handleEscape);
    return () => document.removeEventListener("keydown", handleEscape);
  }, [isResolving, onDecision]);

  function trapFocus(event: KeyboardEvent<HTMLElement>) {
    if (event.key !== "Tab") return;
    const controls = Array.from(dialogRef.current?.querySelectorAll<HTMLElement>(
      "button:not(:disabled)",
    ) ?? []);
    if (controls.length === 0) return;
    const first = controls[0];
    const last = controls.at(-1)!;
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }

  return (
    <div className="command-confirmation-backdrop">
      <section
        aria-describedby="command-confirmation-description"
        aria-labelledby="command-confirmation-title"
        aria-modal="true"
        className="command-confirmation-dialog"
        onKeyDown={trapFocus}
        ref={dialogRef}
        role="alertdialog"
      >
        <header>
          <div>
            <span className="command-confirmation-kicker">M.I.O. SAFETY</span>
            <h2 id="command-confirmation-title">{t("commandConfirmationTitle")}</h2>
          </div>
          <button
            aria-label={t("close")}
            className="command-confirmation-close"
            disabled={isResolving}
            onClick={() => void onDecision("dismissed")}
            type="button"
          >
            ×
          </button>
        </header>

        <p id="command-confirmation-description">
          {t("commandConfirmationSubtitle")}
        </p>

        <dl className="command-confirmation-details">
          <div>
            <dt>{t("commandConfirmationAction")}</dt>
            <dd>{actionText(request.action, t)}</dd>
          </div>
          <div>
            <dt>{t("commandConfirmationReason")}</dt>
            <dd>{reasonText(request.reason, t)}</dd>
          </div>
          <div>
            <dt>{t("commandConfirmationTarget")}</dt>
            <dd><code>{request.targetLabel}</code></dd>
          </div>
        </dl>

        {remainingCount > 0 ? (
          <p className="command-confirmation-more">
            {t("commandConfirmationMore", { count: remainingCount })}
          </p>
        ) : null}
        {error ? (
          <p className="command-confirmation-error" role="alert">
            {t("commandConfirmationFailed")}
          </p>
        ) : null}

        <footer>
          <button
            className="command-confirmation-deny"
            disabled={isResolving}
            onClick={() => void onDecision("deny")}
            ref={denyButtonRef}
            type="button"
          >
            {t("commandConfirmationDeny")}
          </button>
          {request.roomSessionAllowed ? (
            <button
              className="command-confirmation-session"
              disabled={isResolving}
              onClick={() => void onDecision("allowRoomSession")}
              type="button"
            >
              {t("commandConfirmationAllowRoomSession")}
            </button>
          ) : null}
          <button
            className="command-confirmation-once"
            disabled={isResolving}
            onClick={() => void onDecision("allowOnce")}
            type="button"
          >
            {isResolving ? t("commandConfirmationResolving") : t("commandConfirmationAllowOnce")}
          </button>
        </footer>
      </section>
    </div>
  );
}

type Translate = ReturnType<typeof useUiPreferences>["t"];

function actionText(action: CommandConfirmationAction, t: Translate) {
  switch (action) {
    case "gitPush": return t("commandActionGitPush");
    case "packageInstall": return t("commandActionPackageInstall");
    case "cargoFetch": return t("commandActionCargoFetch");
    case "unregisteredTool": return t("commandActionUnregisteredTool");
    case "credentialUse": return t("commandActionCredentialUse");
    case "administratorOperation": return t("commandActionAdministratorOperation");
    case "destructiveOperation": return t("commandActionDestructiveOperation");
  }
}

function reasonText(reason: CommandConfirmationReason, t: Translate) {
  switch (reason) {
    case "externalMutation": return t("commandReasonExternalMutation");
    case "networkDownload": return t("commandReasonNetworkDownload");
    case "unregisteredTool": return t("commandReasonUnregisteredTool");
    case "credentialUse": return t("commandReasonCredentialUse");
    case "privilegeExpansion": return t("commandReasonPrivilegeExpansion");
    case "destructiveChange": return t("commandReasonDestructiveChange");
  }
}
