function toRecipient(participant) {
  return {
    adapterInstanceId: participant.adapterInstanceId,
    displayName: participant.displayName,
  };
}

export function buildDeliveryPlan(participants) {
  const uniqueByAdapter = new Map();

  for (const participant of participants) {
    if (!uniqueByAdapter.has(participant.adapterInstanceId)) {
      uniqueByAdapter.set(participant.adapterInstanceId, participant);
    }
  }

  const selected = [...uniqueByAdapter.values()].filter(
    (participant) => participant.selected,
  );
  const recipients = selected
    .filter((participant) => participant.connection === "connected")
    .map(toRecipient);
  const blockedRecipients = selected
    .filter((participant) => participant.connection !== "connected")
    .map(toRecipient);

  if (selected.length === 0) {
    return {
      status: "blocked",
      reason: "no_recipients",
      warning: null,
      recipients,
      blockedRecipients,
    };
  }

  if (recipients.length === 0) {
    return {
      status: "blocked",
      reason: "all_selected_offline",
      warning: null,
      recipients,
      blockedRecipients,
    };
  }

  return {
    status: "ready",
    reason: null,
    warning:
      blockedRecipients.length > 0 ? "some_selected_offline" : null,
    recipients,
    blockedRecipients,
  };
}
