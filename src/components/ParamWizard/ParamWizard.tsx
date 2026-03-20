import { useState, useCallback, useEffect, useRef } from "react";
import type { CommandDefinition, CommandResult } from "../../lib/types";
import { executeAction } from "../../lib/tauri";
import { ParamInput } from "./ParamInput";
import { Button, ViewContainer } from "../../ui";
import "./ParamWizard.css";

interface ParamWizardProps {
  command: CommandDefinition;
  onComplete: (result: CommandResult) => void;
  onCancel: () => void;
  initialValues?: Record<string, unknown>;
}

export function ParamWizard({
  command,
  onComplete,
  onCancel,
  initialValues,
}: ParamWizardProps) {
  const requiredParams = command.params.filter((p) => p.group === "Required");

  // Compute initial step: skip to first unfilled required param
  const computeInitialStep = () => {
    if (!initialValues || Object.keys(initialValues).length === 0) return 0;
    const firstUnfilled = requiredParams.findIndex(
      (p) => !initialValues[p.id],
    );
    return firstUnfilled === -1 ? requiredParams.length : Math.max(0, firstUnfilled);
  };

  const [step, setStep] = useState(computeInitialStep);
  const [values, setValues] = useState<Record<string, unknown>>(
    initialValues ?? {},
  );
  const [isExecuting, setIsExecuting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const executingRef = useRef(false);

  const currentParam = requiredParams[step];

  const setValue = useCallback(
    (paramId: string, value: unknown) => {
      setValues((prev) => ({ ...prev, [paramId]: value }));
    },
    [],
  );

  const handleExecute = useCallback(async () => {
    if (executingRef.current) return;
    executingRef.current = true;
    setIsExecuting(true);
    setError(null);
    try {
      const result = await executeAction(command.id, values);
      onComplete(result);
    } catch (e) {
      setError(String(e));
      setIsExecuting(false);
      executingRef.current = false;
    }
  }, [command.id, values, onComplete]);

  const handleStepSubmit = useCallback(() => {
    if (step < requiredParams.length - 1) {
      setStep((s) => s + 1);
    } else {
      // All required params filled — execute directly
      handleExecute();
    }
  }, [step, requiredParams.length, handleExecute]);

  // Auto-execute when all required params are already filled (step starts past end)
  useEffect(() => {
    if (step >= requiredParams.length) {
      handleExecute();
    }
  }, []); // Only on mount

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        if (step > 0 && step < requiredParams.length) {
          setStep((s) => s - 1);
        } else {
          onCancel();
        }
      }
    },
    [step, requiredParams.length, onCancel],
  );

  // Attach escape handler
  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  // If executing (all params were pre-filled), show a minimal executing state
  if (step >= requiredParams.length) {
    return (
      <ViewContainer>
        <ViewContainer.Body className="wizard-body">
          {error ? (
            <>
              <div className="wizard-error">{error}</div>
              <div className="wizard-actions">
                <Button onClick={onCancel}>
                  Back
                </Button>
                <Button
                  variant="primary"
                  onClick={() => {
                    executingRef.current = false;
                    handleExecute();
                  }}
                >
                  Retry
                </Button>
              </div>
            </>
          ) : (
            <div className="wizard-executing">Executing...</div>
          )}
        </ViewContainer.Body>
      </ViewContainer>
    );
  }

  return (
    <ViewContainer>
      {requiredParams.length > 1 && (
        <div className="wizard-step-bar">
          {requiredParams.map((_, i) => (
            <div
              key={i}
              className={`wizard-dot ${i < step ? "done" : ""} ${i === step ? "active" : ""}`}
            />
          ))}
        </div>
      )}

      {currentParam && (
        <ViewContainer.Body className="wizard-body">
          <label className="wizard-label">{currentParam.name}</label>
          <ParamInput
            param={currentParam}
            commandId={command.id}
            value={values[currentParam.id]}
            onChange={(v) => setValue(currentParam.id, v)}
            onSubmit={handleStepSubmit}
            autoFocus
          />
        </ViewContainer.Body>
      )}
    </ViewContainer>
  );
}
