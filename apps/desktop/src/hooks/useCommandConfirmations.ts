import { useEffect, useRef, useState } from "react";

import {
  type CommandConfirmationDecision,
  type CommandConfirmationRequest,
  readDesktopCommandConfirmations,
  resolveDesktopCommandConfirmation,
} from "../commandConfirmationBridge";

const refreshIntervalMilliseconds = 500;

export function useCommandConfirmations(input: {
  enabled: boolean;
  roomId: string;
  waiting: boolean;
}) {
  const [requests, setRequests] = useState<CommandConfirmationRequest[]>([]);
  const [error, setError] = useState(false);
  const [isResolving, setResolving] = useState(false);
  const generationRef = useRef(0);
  const refreshGenerationRef = useRef(0);
  const resolvingRef = useRef(false);

  useEffect(() => {
    const generation = ++generationRef.current;
    resolvingRef.current = false;
    if (!input.enabled || !input.waiting) {
      setRequests([]);
      setError(false);
      setResolving(false);
      return;
    }

    let reading = false;
    const refresh = async () => {
      if (reading || resolvingRef.current) return;
      reading = true;
      const refreshGeneration = refreshGenerationRef.current;
      try {
        const next = await readDesktopCommandConfirmations(input.roomId);
        if (
          generation === generationRef.current
          && refreshGeneration === refreshGenerationRef.current
          && !resolvingRef.current
        ) {
          setRequests(next);
          setError(false);
        }
      } catch {
        if (
          generation === generationRef.current
          && refreshGeneration === refreshGenerationRef.current
          && !resolvingRef.current
        ) {
          setError(true);
        }
      } finally {
        reading = false;
      }
    };
    void refresh();
    const interval = window.setInterval(() => void refresh(), refreshIntervalMilliseconds);
    return () => window.clearInterval(interval);
  }, [input.enabled, input.roomId, input.waiting]);

  async function resolve(decision: CommandConfirmationDecision) {
    const request = requests[0];
    if (!request || resolvingRef.current) return false;
    const generation = generationRef.current;
    refreshGenerationRef.current += 1;
    resolvingRef.current = true;
    setResolving(true);
    setError(false);
    try {
      await resolveDesktopCommandConfirmation({
        roomId: input.roomId,
        requestId: request.requestId,
        decision,
      });
      if (generation === generationRef.current) {
        setRequests((current) => current.filter(
          (candidate) => candidate.requestId !== request.requestId,
        ));
      }
      return true;
    } catch {
      if (generation === generationRef.current) setError(true);
      return false;
    } finally {
      if (generation === generationRef.current) {
        resolvingRef.current = false;
        setResolving(false);
      }
    }
  }

  return {
    activeRequest: requests[0] ?? null,
    error,
    isResolving,
    remainingCount: Math.max(0, requests.length - 1),
    resolve,
  };
}
