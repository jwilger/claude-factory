namespace CfkKernel

-- EMC generated Lean4 model root.

def modelName := "cfk-kernel"

def modelVersion := "0.1.0"

def modelDigest := "72fb303634e722acb3e503ae5a63a22cc9aa0ec51c297bcd45d6db6e2655f78b"

structure ModelWorkflow where
  workflow : String

def modelWorkflows : List ModelWorkflow := [{ workflow := "continue-noninteractive-work-during-open-gate" },{ workflow := "declare-worktree-provisioning-config" },{ workflow := "first-run-no-snapshot-migration" },{ workflow := "inspect-pending-decisions" },{ workflow := "kernel-defers-contending-gate" },{ workflow := "kernel-opens-interactive-gate-acquires-lock" },{ workflow := "kernel-resolves-releases-lock" },{ workflow := "noop-hermetic-projects" },{ workflow := "operator-approves-gate" },{ workflow := "operator-rejects-gate" },{ workflow := "present-ask-human-gate-with-context" },{ workflow := "provision-worktree-on-creation" },{ workflow := "record-what-was-provisioned" },{ workflow := "relax-guardrail-under-multi-session" },{ workflow := "replay-cost-instrumentation" },{ workflow := "run-agent-in-ready-environment" },{ workflow := "run-setup-escape-hatch-hook" },{ workflow := "snapshot-accelerated-loader" },{ workflow := "snapshot-creation-trigger" },{ workflow := "snapshot-versioning-invalidation" },{ workflow := "tear-down-worktree-clean-secrets" },{ workflow := "validate-provisioning-config-on-load" }]

structure ModelSlice where
  workflow : String
  slice : String

structure ModelSliceModule where
  workflow : String
  slice : String
  formalModule : String

structure ModelScenario where
  workflow : String
  slice : String
  scenarioKind : String
  scenario : String

structure ModelScenarioDefinition where
  workflow : String
  slice : String
  scenarioKind : String
  scenario : String
  given : String
  when : String
  thenStep : String
  readStreams : List String
  writtenStreams : List String
  contractKind : String
  coveredDefinition : String
  errorReferences : List String

inductive ModelDataFlowSourceKind where
  | original
  | modeledTarget
deriving BEq, DecidableEq, Repr

structure ModelDataFlow where
  workflow : String
  slice : String
  datum : String
  sourceKind : ModelDataFlowSourceKind
  source : String
  transformation : String
  target : String
  bitEncoding : String

structure ModelOutcome where
  workflow : String
  slice : String
  outcome : String
  events : List String
  externallyRelevant : Bool

structure ModelCommandError where
  workflow : String
  slice : String
  command : String
  error : String
  scenario : String
  recovery : String

structure ModelCommand where
  workflow : String
  slice : String
  command : String

inductive ModelCommandInputSourceKind where
  | actor
  | session
  | generated
  | externalPayload
  | eventStreamState
  | invocationArgument
deriving BEq, DecidableEq, Repr

structure ModelCommandInput where
  workflow : String
  slice : String
  command : String
  input : String
  sourceKind : ModelCommandInputSourceKind
  sourceDescription : String
  provenanceChain : List String
  eventStreamSourceEvent : String
  eventStreamSourceAttribute : String
  externalPayloadSourceName : String
  externalPayloadSourceField : String
  generatedSourceName : String
  generatedSourceField : String
  sessionSourceName : String
  sessionSourceField : String
  invocationArgumentSourceName : String
  invocationArgumentSourceField : String

structure ModelReadModel where
  workflow : String
  slice : String
  readModel : String

structure ModelReadModelDefinition where
  workflow : String
  slice : String
  readModel : String
  transitive : Bool
  relationshipFields : List String
  transitiveRule : String
  exampleScenarioName : String

structure ModelReadModelField where
  workflow : String
  slice : String
  readModel : String
  field : String
  sourceKind : String
  sourceEvent : String
  sourceAttribute : String
  derivationRule : String
  derivationSourceFields : List String
  absenceEvent : String
  derivationScenarioName : String
  absenceScenarioName : String
  provenance : String

structure ModelView where
  workflow : String
  slice : String
  view : String

structure ModelViewDefinition where
  workflow : String
  slice : String
  view : String
  readModels : List String
  sketchTokens : List String
  localStates : List String
  filters : List String

structure ModelViewControl where
  workflow : String
  slice : String
  view : String
  control : String
  command : String
  input : String
  inputSourceKind : ModelCommandInputSourceKind
  inputSourceDescription : String
  inputSketchToken : String
  inputVisibleToActor : Bool
  inputDecisionField : Bool
  handledErrors : List String
  recoveryBehavior : String
  controlSketchToken : String
  navigationType : String
  navigationTarget : String
  externalWorkflow : String
  externalSystem : String
  handoffContract : String

structure ModelViewField where
  workflow : String
  slice : String
  view : String
  field : String
  sourceKind : String
  sourceReadModel : String
  sourceField : String
  provenance : String
  bitEncoding : String

structure ModelBoardElement where
  workflow : String
  slice : String
  element : String
  kind : String
  lane : String
  declaredName : String
  mainPath : Bool

structure ModelBoardConnection where
  workflow : String
  slice : String
  source : String
  sourceKind : String
  target : String
  targetKind : String

structure ModelAutomation where
  workflow : String
  slice : String
  automation : String

structure ModelAutomationDefinition where
  workflow : String
  slice : String
  automation : String
  trigger : String
  command : String
  handledErrors : List String
  reaction : String

structure ModelTranslation where
  workflow : String
  slice : String
  translation : String

structure ModelTranslationDefinition where
  workflow : String
  slice : String
  translation : String
  externalEvent : String
  payloadContract : String
  command : String

structure ModelExternalPayload where
  workflow : String
  slice : String
  externalPayload : String

structure ModelExternalPayloadField where
  workflow : String
  slice : String
  externalPayload : String
  field : String
  provenance : String
  bitEncoding : String

structure ModelStream where
  workflow : String
  slice : String
  stream : String

structure ModelEvent where
  workflow : String
  slice : String
  event : String
  stream : String

structure ModelEventAttribute where
  workflow : String
  slice : String
  event : String
  attributeName : String
  sourceKind : String
  sourceName : String
  sourceField : String
  generatedSourceKind : String
  provenance : String

def modelSlices : List ModelSlice := [{ workflow := "continue-noninteractive-work-during-open-gate", slice := "execute-non-interactive-step-concurrently" },{ workflow := "continue-noninteractive-work-during-open-gate", slice := "expose-concurrent-progress-in-status" },{ workflow := "continue-noninteractive-work-during-open-gate", slice := "schedule-non-interactive-step-while-gate-open" },{ workflow := "declare-worktree-provisioning-config", slice := "set-worktree-provisioning-config" },{ workflow := "declare-worktree-provisioning-config", slice := "view-worktree-config-in-status" },{ workflow := "first-run-no-snapshot-migration", slice := "detect-no-snapshot-on-startup" },{ workflow := "first-run-no-snapshot-migration", slice := "first-run-empty-store-init" },{ workflow := "first-run-no-snapshot-migration", slice := "full-event-replay-from-origin" },{ workflow := "inspect-pending-decisions", slice := "expose-pending-decisions-in-status" },{ workflow := "inspect-pending-decisions", slice := "remove-decision-on-answer" },{ workflow := "inspect-pending-decisions", slice := "track-unanswered-human-decisions" },{ workflow := "kernel-defers-contending-gate", slice := "detect-gate-lock-contention" },{ workflow := "kernel-defers-contending-gate", slice := "emit-gate-deferred" },{ workflow := "kernel-defers-contending-gate", slice := "reenqueue-deferred-gate-on-lock-release" },{ workflow := "kernel-opens-interactive-gate-acquires-lock", slice := "acquire-gate-lock-before-opening" },{ workflow := "kernel-opens-interactive-gate-acquires-lock", slice := "open-interactive-gate-with-lock-held" },{ workflow := "kernel-opens-interactive-gate-acquires-lock", slice := "release-gate-lock-on-verdict" },{ workflow := "kernel-resolves-releases-lock", slice := "force-release-lock-on-timeout" },{ workflow := "kernel-resolves-releases-lock", slice := "release-lock-after-verdict" },{ workflow := "kernel-resolves-releases-lock", slice := "view-gate-lock-state-in-status" },{ workflow := "noop-hermetic-projects", slice := "detect-hermetic-project-flag" },{ workflow := "noop-hermetic-projects", slice := "emit-worktree-provision-skipped" },{ workflow := "noop-hermetic-projects", slice := "expose-hermetic-skip-in-status" },{ workflow := "operator-approves-gate", slice := "advance-work-item-to-done-on-approval" },{ workflow := "operator-approves-gate", slice := "expose-completed-item-in-status" },{ workflow := "operator-approves-gate", slice := "reviewer-submits-approval" },{ workflow := "operator-rejects-gate", slice := "expose-rework-item-in-backlog" },{ workflow := "operator-rejects-gate", slice := "return-work-item-for-rework" },{ workflow := "operator-rejects-gate", slice := "reviewer-submits-veto" },{ workflow := "present-ask-human-gate-with-context", slice := "assemble-human-gate-context-header" },{ workflow := "present-ask-human-gate-with-context", slice := "present-human-decision-request" },{ workflow := "present-ask-human-gate-with-context", slice := "record-human-decision" },{ workflow := "provision-worktree-on-creation", slice := "create-git-worktree" },{ workflow := "provision-worktree-on-creation", slice := "emit-provision-worktree-command" },{ workflow := "provision-worktree-on-creation", slice := "expose-worktree-path-in-step-context" },{ workflow := "record-what-was-provisioned", slice := "emit-provisioning-complete-record" },{ workflow := "record-what-was-provisioned", slice := "view-active-worktree-inventory" },{ workflow := "relax-guardrail-under-multi-session", slice := "block-cross-session-worktree-edit" },{ workflow := "relax-guardrail-under-multi-session", slice := "permit-edit-for-leased-session" },{ workflow := "relax-guardrail-under-multi-session", slice := "verify-session-lease-matches-worktree" },{ workflow := "replay-cost-instrumentation", slice := "expose-replay-metrics" },{ workflow := "replay-cost-instrumentation", slice := "measure-replay-cost" },{ workflow := "replay-cost-instrumentation", slice := "snapshot-threshold-watch" },{ workflow := "run-agent-in-ready-environment", slice := "agent-executes-tdd-work-in-worktree" },{ workflow := "run-agent-in-ready-environment", slice := "inject-worktree-context-into-step" },{ workflow := "run-agent-in-ready-environment", slice := "submit-result-advancing-tdd-state" },{ workflow := "run-setup-escape-hatch-hook", slice := "detect-setup-hook-script" },{ workflow := "run-setup-escape-hatch-hook", slice := "execute-setup-hook-in-worktree" },{ workflow := "run-setup-escape-hatch-hook", slice := "expose-hook-result-in-step-context" },{ workflow := "snapshot-accelerated-loader", slice := "detect-available-snapshot" },{ workflow := "snapshot-accelerated-loader", slice := "load-state-from-snapshot" },{ workflow := "snapshot-accelerated-loader", slice := "replay-delta-events" },{ workflow := "snapshot-creation-trigger", slice := "create-snapshot" },{ workflow := "snapshot-creation-trigger", slice := "expose-snapshot-metadata" },{ workflow := "snapshot-creation-trigger", slice := "trigger-snapshot-on-threshold" },{ workflow := "snapshot-versioning-invalidation", slice := "check-snapshot-schema-version" },{ workflow := "snapshot-versioning-invalidation", slice := "fallback-full-replay-after-invalidation" },{ workflow := "snapshot-versioning-invalidation", slice := "invalidate-incompatible-snapshot" },{ workflow := "tear-down-worktree-clean-secrets", slice := "remove-git-worktree" },{ workflow := "tear-down-worktree-clean-secrets", slice := "run-teardown-hook-clean-secrets" },{ workflow := "tear-down-worktree-clean-secrets", slice := "trigger-teardown-on-terminal-state" },{ workflow := "validate-provisioning-config-on-load", slice := "expose-config-validity-in-status" },{ workflow := "validate-provisioning-config-on-load", slice := "restore-worktree-config-from-events" },{ workflow := "validate-provisioning-config-on-load", slice := "validate-config-coherence-on-startup" }]

def modelSliceModules : List ModelSliceModule := [{ workflow := "continue-noninteractive-work-during-open-gate", slice := "execute-non-interactive-step-concurrently", formalModule := "ExecuteNonInteractiveStepConcurrently" },{ workflow := "continue-noninteractive-work-during-open-gate", slice := "expose-concurrent-progress-in-status", formalModule := "ExposeConcurrentProgressInCfStatus" },{ workflow := "continue-noninteractive-work-during-open-gate", slice := "schedule-non-interactive-step-while-gate-open", formalModule := "ScheduleNonInteractiveStepWhileGateIsOpen" },{ workflow := "declare-worktree-provisioning-config", slice := "set-worktree-provisioning-config", formalModule := "SetWorktreeProvisioningConfiguration" },{ workflow := "declare-worktree-provisioning-config", slice := "view-worktree-config-in-status", formalModule := "ViewWorktreeConfigInCfStatus" },{ workflow := "first-run-no-snapshot-migration", slice := "detect-no-snapshot-on-startup", formalModule := "DetectNoSnapshotOnStartup" },{ workflow := "first-run-no-snapshot-migration", slice := "first-run-empty-store-init", formalModule := "FirstRunEmptyStoreInitialisation" },{ workflow := "first-run-no-snapshot-migration", slice := "full-event-replay-from-origin", formalModule := "FullEventReplayFromOrigin" },{ workflow := "inspect-pending-decisions", slice := "expose-pending-decisions-in-status", formalModule := "ExposePendingDecisionsInCfStatus" },{ workflow := "inspect-pending-decisions", slice := "remove-decision-on-answer", formalModule := "RemoveDecisionFromPendingListOnAnswer" },{ workflow := "inspect-pending-decisions", slice := "track-unanswered-human-decisions", formalModule := "TrackUnansweredHumanDecisionsInReadModel" },{ workflow := "kernel-defers-contending-gate", slice := "detect-gate-lock-contention", formalModule := "DetectGateLockContention" },{ workflow := "kernel-defers-contending-gate", slice := "emit-gate-deferred", formalModule := "EmitGateDeferredForContendingWorkItem" },{ workflow := "kernel-defers-contending-gate", slice := "reenqueue-deferred-gate-on-lock-release", formalModule := "ReEnqueueDeferredGateWhenLockReleased" },{ workflow := "kernel-opens-interactive-gate-acquires-lock", slice := "acquire-gate-lock-before-opening", formalModule := "AcquireGateLockBeforeOpeningInteractiveGate" },{ workflow := "kernel-opens-interactive-gate-acquires-lock", slice := "open-interactive-gate-with-lock-held", formalModule := "OpenInteractiveGateWithLockHeld" },{ workflow := "kernel-opens-interactive-gate-acquires-lock", slice := "release-gate-lock-on-verdict", formalModule := "ReleaseGateLockAfterVerdictRecorded" },{ workflow := "kernel-resolves-releases-lock", slice := "force-release-lock-on-timeout", formalModule := "ForceReleaseLockOnTimeout" },{ workflow := "kernel-resolves-releases-lock", slice := "release-lock-after-verdict", formalModule := "ReleaseLockAfterGateVerdictRecorded" },{ workflow := "kernel-resolves-releases-lock", slice := "view-gate-lock-state-in-status", formalModule := "ViewGateLockStateInCfStatus" },{ workflow := "noop-hermetic-projects", slice := "detect-hermetic-project-flag", formalModule := "DetectHermeticProjectConfiguration" },{ workflow := "noop-hermetic-projects", slice := "emit-worktree-provision-skipped", formalModule := "EmitWorktreeProvisionSkippedForHermeticProject" },{ workflow := "noop-hermetic-projects", slice := "expose-hermetic-skip-in-status", formalModule := "ExposeHermeticSkipInCfStatus" },{ workflow := "operator-approves-gate", slice := "advance-work-item-to-done-on-approval", formalModule := "AdvanceWorkItemToDoneOnApproval" },{ workflow := "operator-approves-gate", slice := "expose-completed-item-in-status", formalModule := "ExposeCompletedWorkItemInCfStatus" },{ workflow := "operator-approves-gate", slice := "reviewer-submits-approval", formalModule := "ReviewerSubmitsApprovalViaCfGate" },{ workflow := "operator-rejects-gate", slice := "expose-rework-item-in-backlog", formalModule := "ExposeReworkItemInCfStatusBacklog" },{ workflow := "operator-rejects-gate", slice := "return-work-item-for-rework", formalModule := "ReturnWorkItemForReworkAfterVeto" },{ workflow := "operator-rejects-gate", slice := "reviewer-submits-veto", formalModule := "ReviewerSubmitsVetoViaCfGate" },{ workflow := "present-ask-human-gate-with-context", slice := "assemble-human-gate-context-header", formalModule := "AssembleContextHeaderForHumanGate" },{ workflow := "present-ask-human-gate-with-context", slice := "present-human-decision-request", formalModule := "PresentHumanDecisionRequestToOperator" },{ workflow := "present-ask-human-gate-with-context", slice := "record-human-decision", formalModule := "RecordHumanDecision" },{ workflow := "provision-worktree-on-creation", slice := "create-git-worktree", formalModule := "CreateGitWorktree" },{ workflow := "provision-worktree-on-creation", slice := "emit-provision-worktree-command", formalModule := "EmitProvisionWorktreeCommandOnClaim" },{ workflow := "provision-worktree-on-creation", slice := "expose-worktree-path-in-step-context", formalModule := "ExposeWorktreePathInStepContext" },{ workflow := "record-what-was-provisioned", slice := "emit-provisioning-complete-record", formalModule := "EmitProvisioningCompleteRecord" },{ workflow := "record-what-was-provisioned", slice := "view-active-worktree-inventory", formalModule := "ViewActiveWorktreeInventory" },{ workflow := "relax-guardrail-under-multi-session", slice := "block-cross-session-worktree-edit", formalModule := "BlockCrossSessionWorktreeEditAttempt" },{ workflow := "relax-guardrail-under-multi-session", slice := "permit-edit-for-leased-session", formalModule := "PermitEditForSessionWithValidWorktreeLease" },{ workflow := "relax-guardrail-under-multi-session", slice := "verify-session-lease-matches-worktree", formalModule := "VerifySessionLeaseMatchesTargetWorktree" },{ workflow := "replay-cost-instrumentation", slice := "expose-replay-metrics", formalModule := "ExposeReplayMetricsInCfStatus" },{ workflow := "replay-cost-instrumentation", slice := "measure-replay-cost", formalModule := "MeasureReplayCostOnStartup" },{ workflow := "replay-cost-instrumentation", slice := "snapshot-threshold-watch", formalModule := "WarnWhenSnapshotThresholdCrossed" },{ workflow := "run-agent-in-ready-environment", slice := "agent-executes-tdd-work-in-worktree", formalModule := "AgentExecutesTDDWorkInWorktree" },{ workflow := "run-agent-in-ready-environment", slice := "inject-worktree-context-into-step", formalModule := "InjectWorktreeContextIntoStepPrompt" },{ workflow := "run-agent-in-ready-environment", slice := "submit-result-advancing-tdd-state", formalModule := "SubmitResultAdvancingTDDStateMachine" },{ workflow := "run-setup-escape-hatch-hook", slice := "detect-setup-hook-script", formalModule := "DetectSetupHookScriptAfterProvisioning" },{ workflow := "run-setup-escape-hatch-hook", slice := "execute-setup-hook-in-worktree", formalModule := "ExecuteSetupHookInWorktree" },{ workflow := "run-setup-escape-hatch-hook", slice := "expose-hook-result-in-step-context", formalModule := "ExposeHookResultInStepContext" },{ workflow := "snapshot-accelerated-loader", slice := "detect-available-snapshot", formalModule := "DetectAvailableSnapshotOnColdStart" },{ workflow := "snapshot-accelerated-loader", slice := "load-state-from-snapshot", formalModule := "LoadStateFromSnapshot" },{ workflow := "snapshot-accelerated-loader", slice := "replay-delta-events", formalModule := "ReplayDeltaEventsSinceSnapshot" },{ workflow := "snapshot-creation-trigger", slice := "create-snapshot", formalModule := "WriteSnapshotToStorage" },{ workflow := "snapshot-creation-trigger", slice := "expose-snapshot-metadata", formalModule := "ExposeSnapshotMetadataInCfStatus" },{ workflow := "snapshot-creation-trigger", slice := "trigger-snapshot-on-threshold", formalModule := "TriggerSnapshotWhenReplayThresholdCrossed" },{ workflow := "snapshot-versioning-invalidation", slice := "check-snapshot-schema-version", formalModule := "CheckSnapshotSchemaVersionOnLoad" },{ workflow := "snapshot-versioning-invalidation", slice := "fallback-full-replay-after-invalidation", formalModule := "FallBackToFullReplayAfterInvalidation" },{ workflow := "snapshot-versioning-invalidation", slice := "invalidate-incompatible-snapshot", formalModule := "InvalidateIncompatibleSnapshot" },{ workflow := "tear-down-worktree-clean-secrets", slice := "remove-git-worktree", formalModule := "RemoveGitWorktree" },{ workflow := "tear-down-worktree-clean-secrets", slice := "run-teardown-hook-clean-secrets", formalModule := "RunTeardownHookToCleanSecrets" },{ workflow := "tear-down-worktree-clean-secrets", slice := "trigger-teardown-on-terminal-state", formalModule := "TriggerTeardownOnTerminalWorkItemState" },{ workflow := "validate-provisioning-config-on-load", slice := "expose-config-validity-in-status", formalModule := "ExposeConfigValidityInCfStatus" },{ workflow := "validate-provisioning-config-on-load", slice := "restore-worktree-config-from-events", formalModule := "RestoreWorktreeConfigFromEventReplay" },{ workflow := "validate-provisioning-config-on-load", slice := "validate-config-coherence-on-startup", formalModule := "ValidateConfigCoherenceOnStartup" }]

def modelSliceBelongsToDeclaredWorkflow (slice : ModelSlice) : Bool := modelWorkflows.any (fun workflow => workflow.workflow == slice.workflow)

def modelSliceHasModule (slice : ModelSlice) : Bool := modelSliceModules.any (fun sliceModule => sliceModule.workflow == slice.workflow && sliceModule.slice == slice.slice && sliceModule.formalModule.isEmpty == false)

def modelSliceModuleBelongsToDeclaredSlice (sliceModule : ModelSliceModule) : Bool := sliceModule.formalModule.isEmpty == false && modelSlices.any (fun slice => slice.workflow == sliceModule.workflow && slice.slice == sliceModule.slice)

def modelWorkflowSlicesHaveModules (workflow : ModelWorkflow) : Bool := modelSlices.all (fun slice => slice.workflow != workflow.workflow || modelSliceHasModule slice)

def modelWorkflowHasCompositionStructure (workflow : ModelWorkflow) : Bool := modelWorkflowSlicesHaveModules workflow

def modelScenarios : List ModelScenario := []

def modelScenarioDefinitions : List ModelScenarioDefinition := []

def modelDataFlows : List ModelDataFlow := []

def modelOutcomes : List ModelOutcome := []

def modelCommandErrors : List ModelCommandError := []

def modelCommands : List ModelCommand := []

def modelCommandInputs : List ModelCommandInput := []

def modelReadModels : List ModelReadModel := []

def modelReadModelDefinitions : List ModelReadModelDefinition := []

def modelReadModelFields : List ModelReadModelField := []

def modelViews : List ModelView := []

def modelViewDefinitions : List ModelViewDefinition := []

def modelViewControls : List ModelViewControl := []

def modelBoardElements : List ModelBoardElement := []

def modelBoardConnections : List ModelBoardConnection := []

def modelViewFields : List ModelViewField := []

def modelAutomations : List ModelAutomation := []

def modelAutomationDefinitions : List ModelAutomationDefinition := []

def modelTranslations : List ModelTranslation := []

def modelTranslationDefinitions : List ModelTranslationDefinition := []

def modelExternalPayloads : List ModelExternalPayload := []

def modelExternalPayloadFields : List ModelExternalPayloadField := []

def modelStreams : List ModelStream := []

def modelEvents : List ModelEvent := []

def modelEventAttributes : List ModelEventAttribute := []

def modelScenarioDefinitionHasGwt (scenario : ModelScenarioDefinition) : Bool := scenario.given.isEmpty == false && scenario.when.isEmpty == false && scenario.thenStep.isEmpty == false

def modelScenarioKindIsFirstClass (scenario : ModelScenarioDefinition) : Bool := scenario.scenarioKind == "acceptance" || scenario.scenarioKind == "contract"

def modelDataFlowIsBitComplete (dataFlow : ModelDataFlow) : Bool := dataFlow.datum.isEmpty == false && dataFlow.source.isEmpty == false && dataFlow.transformation.isEmpty == false && dataFlow.target.isEmpty == false && dataFlow.bitEncoding.isEmpty == false

def modelDataFlowCoversDatumTarget (workflow : String) (slice : String) (datum : String) (target : String) : Bool := modelDataFlows.any (fun dataFlow => dataFlow.workflow == workflow && dataFlow.slice == slice && dataFlow.datum == datum && dataFlow.target == target && modelDataFlowIsBitComplete dataFlow)

def modelDataFlowBitEncodingMatchesDatumTarget (workflow : String) (slice : String) (datum : String) (target : String) (bitEncoding : String) : Bool := modelDataFlows.any (fun dataFlow => dataFlow.workflow == workflow && dataFlow.slice == slice && dataFlow.datum == datum && dataFlow.target == target && dataFlow.bitEncoding == bitEncoding && modelDataFlowIsBitComplete dataFlow)

def modelDataFlowSourceBitEncodingMatchesModeledSource (dataFlow : ModelDataFlow) : Bool := (modelDataFlows.any (fun sourceFlow => sourceFlow.workflow == dataFlow.workflow && sourceFlow.slice == dataFlow.slice && sourceFlow.datum == dataFlow.datum && sourceFlow.target == dataFlow.source) == false) || modelDataFlows.any (fun sourceFlow => sourceFlow.workflow == dataFlow.workflow && sourceFlow.slice == dataFlow.slice && sourceFlow.datum == dataFlow.datum && sourceFlow.target == dataFlow.source && sourceFlow.bitEncoding == dataFlow.bitEncoding && modelDataFlowIsBitComplete sourceFlow)

def modelDataFlowHasModeledTransformationSemantics (dataFlow : ModelDataFlow) : Bool := dataFlow.transformation == "identity" || dataFlow.transformation == "projection" || dataFlow.transformation == "derivation" || dataFlow.transformation == "default" || dataFlow.transformation == "absence" || dataFlow.transformation == "transformation"

def modelDataFlowHasModeledSourceKind (dataFlow : ModelDataFlow) : Bool := match dataFlow.sourceKind with
  | ModelDataFlowSourceKind.original => dataFlow.source.isEmpty == false
  | ModelDataFlowSourceKind.modeledTarget => dataFlow.source.isEmpty == false

def modelDataFlowModeledSourceResolves (dataFlow : ModelDataFlow) : Bool := dataFlow.sourceKind != ModelDataFlowSourceKind.modeledTarget || modelDataFlows.any (fun sourceFlow => sourceFlow.workflow == dataFlow.workflow && sourceFlow.slice == dataFlow.slice && sourceFlow.datum == dataFlow.datum && sourceFlow.target == dataFlow.source && modelDataFlowIsBitComplete sourceFlow)

def modelSameDataFlowTarget (left : ModelDataFlow) (right : ModelDataFlow) : Bool := left.workflow == right.workflow && left.slice == right.slice && left.datum == right.datum && left.target == right.target

def modelDataFlowTargetsFromReachable (reachable : List ModelDataFlow) : List ModelDataFlow := modelDataFlows.filter (fun dataFlow => dataFlow.sourceKind == ModelDataFlowSourceKind.modeledTarget && reachable.any (fun sourceFlow => sourceFlow.workflow == dataFlow.workflow && sourceFlow.slice == dataFlow.slice && sourceFlow.datum == dataFlow.datum && sourceFlow.target == dataFlow.source && modelDataFlowIsBitComplete sourceFlow))

def modelDataFlowsReachableFromOriginalsAfterFuel : Nat -> List ModelDataFlow -> List ModelDataFlow
  | Nat.zero, reachable => reachable
  | Nat.succ fuel, reachable => modelDataFlowsReachableFromOriginalsAfterFuel fuel (reachable ++ modelDataFlowTargetsFromReachable reachable)

def modelDataFlowsReachableFromOriginals : List ModelDataFlow := modelDataFlowsReachableFromOriginalsAfterFuel modelDataFlows.length (modelDataFlows.filter (fun dataFlow => dataFlow.sourceKind == ModelDataFlowSourceKind.original && modelDataFlowIsBitComplete dataFlow))

def modelDataFlowHasOriginalSourceChain (dataFlow : ModelDataFlow) : Bool := dataFlow.sourceKind == ModelDataFlowSourceKind.original || modelDataFlowsReachableFromOriginals.any (fun reachableFlow => modelSameDataFlowTarget reachableFlow dataFlow)

def modelDataFlowTargetsFromBitPreservingReachable (reachable : List ModelDataFlow) : List ModelDataFlow := modelDataFlows.filter (fun dataFlow => dataFlow.sourceKind == ModelDataFlowSourceKind.modeledTarget && reachable.any (fun sourceFlow => sourceFlow.workflow == dataFlow.workflow && sourceFlow.slice == dataFlow.slice && sourceFlow.datum == dataFlow.datum && sourceFlow.target == dataFlow.source && sourceFlow.bitEncoding == dataFlow.bitEncoding && modelDataFlowIsBitComplete sourceFlow))

def modelDataFlowsReachableFromOriginalsWithPreservedBitsAfterFuel : Nat -> List ModelDataFlow -> List ModelDataFlow
  | Nat.zero, reachable => reachable
  | Nat.succ fuel, reachable => modelDataFlowsReachableFromOriginalsWithPreservedBitsAfterFuel fuel (reachable ++ modelDataFlowTargetsFromBitPreservingReachable reachable)

def modelDataFlowsReachableFromOriginalsWithPreservedBits : List ModelDataFlow := modelDataFlowsReachableFromOriginalsWithPreservedBitsAfterFuel modelDataFlows.length (modelDataFlows.filter (fun dataFlow => dataFlow.sourceKind == ModelDataFlowSourceKind.original && modelDataFlowIsBitComplete dataFlow))

def modelDataFlowHasBitPreservingOriginalSourceChain (dataFlow : ModelDataFlow) : Bool := dataFlow.sourceKind == ModelDataFlowSourceKind.original || modelDataFlowsReachableFromOriginalsWithPreservedBits.any (fun reachableFlow => modelSameDataFlowTarget reachableFlow dataFlow)

def modelCommandInputHasModeledDataFlow (input : ModelCommandInput) : Bool := modelDataFlowCoversDatumTarget input.workflow input.slice input.input input.command

def modelEventAttributeHasModeledDataFlow (eventAttribute : ModelEventAttribute) : Bool := modelDataFlowCoversDatumTarget eventAttribute.workflow eventAttribute.slice eventAttribute.attributeName eventAttribute.event

def modelReadModelFieldHasModeledDataFlow (field : ModelReadModelField) : Bool := modelDataFlowCoversDatumTarget field.workflow field.slice field.field field.readModel

def modelViewFieldHasModeledDataFlow (field : ModelViewField) : Bool := modelDataFlowCoversDatumTarget field.workflow field.slice field.field field.view

def modelViewFieldBitEncodingMatchesDataFlow (field : ModelViewField) : Bool := modelDataFlowBitEncodingMatchesDatumTarget field.workflow field.slice field.field field.view field.bitEncoding

def modelExternalPayloadFieldHasModeledDataFlow (field : ModelExternalPayloadField) : Bool := modelDataFlowCoversDatumTarget field.workflow field.slice field.field field.externalPayload

def modelExternalPayloadFieldBitEncodingMatchesDataFlow (field : ModelExternalPayloadField) : Bool := modelDataFlowBitEncodingMatchesDatumTarget field.workflow field.slice field.field field.externalPayload field.bitEncoding

def modelMeaningfulDataHasModeledDataFlows : Bool := modelCommandInputs.all modelCommandInputHasModeledDataFlow && modelEventAttributes.all modelEventAttributeHasModeledDataFlow && modelReadModelFields.all modelReadModelFieldHasModeledDataFlow && modelViewFields.all modelViewFieldHasModeledDataFlow && modelExternalPayloadFields.all modelExternalPayloadFieldHasModeledDataFlow

def modelCommandInputHasProvenance (input : ModelCommandInput) : Bool := input.sourceDescription.isEmpty == false && input.provenanceChain.isEmpty == false

def modelCommandInputTracesToInvocationSource (input : ModelCommandInput) : Bool := input.sourceKind == ModelCommandInputSourceKind.actor || (input.sourceKind == ModelCommandInputSourceKind.eventStreamState && input.eventStreamSourceEvent.isEmpty == false && input.eventStreamSourceAttribute.isEmpty == false) || (input.sourceKind == ModelCommandInputSourceKind.externalPayload && input.externalPayloadSourceName.isEmpty == false && input.externalPayloadSourceField.isEmpty == false) || (input.sourceKind == ModelCommandInputSourceKind.generated && input.generatedSourceName.isEmpty == false && input.generatedSourceField.isEmpty == false) || (input.sourceKind == ModelCommandInputSourceKind.session && input.sessionSourceName.isEmpty == false && input.sessionSourceField.isEmpty == false) || (input.sourceKind == ModelCommandInputSourceKind.invocationArgument && input.invocationArgumentSourceName.isEmpty == false && input.invocationArgumentSourceField.isEmpty == false)

def modelEventAttributeSourceIsComplete (eventAttribute : ModelEventAttribute) : Bool := eventAttribute.provenance.isEmpty == false && ((eventAttribute.sourceKind == "command_input" && eventAttribute.sourceName.isEmpty == false && eventAttribute.sourceField.isEmpty == false) || (eventAttribute.sourceKind == "external_payload" && eventAttribute.sourceName.isEmpty == false && eventAttribute.sourceField.isEmpty == false) || (eventAttribute.sourceKind == "generated" && eventAttribute.sourceName.isEmpty == false && eventAttribute.generatedSourceKind.isEmpty == false) || (eventAttribute.sourceKind == "session" && eventAttribute.sourceName.isEmpty == false) || (eventAttribute.sourceKind == "derivation" && eventAttribute.sourceName.isEmpty == false && eventAttribute.sourceField.isEmpty == false))

def modelReadModelFieldSourceIsComplete (field : ModelReadModelField) : Bool := (field.sourceKind == "event_attribute" && field.sourceEvent.isEmpty == false && field.sourceAttribute.isEmpty == false) || (field.sourceKind == "derivation" && field.derivationRule.isEmpty == false && field.derivationSourceFields.isEmpty == false) || (field.sourceKind == "absence_default" && field.absenceEvent.isEmpty == false)

def modelReadModelFieldTracesToOriginalProvenance (field : ModelReadModelField) : Bool := field.provenance.isEmpty == false && ((field.sourceKind == "event_attribute" && modelEventAttributes.any (fun eventAttribute => eventAttribute.workflow == field.workflow && eventAttribute.slice == field.slice && eventAttribute.event == field.sourceEvent && eventAttribute.attributeName == field.sourceAttribute && modelEventAttributeSourceIsComplete eventAttribute)) || (field.sourceKind == "derivation" && field.derivationRule.isEmpty == false && field.derivationSourceFields.isEmpty == false) || (field.sourceKind == "absence_default" && field.absenceEvent.isEmpty == false))

def modelViewFieldSourceIsComplete (field : ModelViewField) : Bool := field.sourceKind == "read_model" && field.sourceReadModel.isEmpty == false && field.sourceField.isEmpty == false && field.provenance.isEmpty == false && field.bitEncoding.isEmpty == false

def modelViewFieldReadModelFieldSourceResolves (viewField : ModelViewField) : Bool := modelViewFieldSourceIsComplete viewField && modelReadModelFields.any (fun readModelField => readModelField.workflow == viewField.workflow && readModelField.slice == viewField.slice && readModelField.readModel == viewField.sourceReadModel && readModelField.field == viewField.sourceField && modelReadModelFieldSourceIsComplete readModelField)

def modelDisplayedDatumTracesToOriginalProvenance (viewField : ModelViewField) : Bool := modelViewFieldReadModelFieldSourceResolves viewField && modelReadModelFields.any (fun readModelField => readModelField.workflow == viewField.workflow && readModelField.slice == viewField.slice && readModelField.readModel == viewField.sourceReadModel && readModelField.field == viewField.sourceField && modelReadModelFieldTracesToOriginalProvenance readModelField)

def modelExternalPayloadFieldHasProvenance (field : ModelExternalPayloadField) : Bool := field.provenance.isEmpty == false && field.bitEncoding.isEmpty == false

def modelControlProvidesCommandInput (control : ModelViewControl) (input : ModelCommandInput) : Bool := control.workflow == input.workflow && control.command == input.command && control.input == input.input

def modelViewControlProvidesEveryCommandInput (control : ModelViewControl) : Bool := modelCommandInputs.all (fun input => input.workflow != control.workflow || input.command != control.command || modelViewControls.any (fun providedInput => providedInput.workflow == control.workflow && providedInput.slice == control.slice && providedInput.view == control.view && providedInput.control == control.control && providedInput.command == control.command && modelControlProvidesCommandInput providedInput input))

def modelOutcomeBranchIsModeled (outcome : ModelOutcome) : Bool := outcome.outcome.isEmpty == false && outcome.events.isEmpty == false

def modelCommandErrorRecoveryIsModeled (commandError : ModelCommandError) : Bool := commandError.command.isEmpty == false && commandError.error.isEmpty == false && commandError.scenario.isEmpty == false && commandError.recovery.isEmpty == false

def modelViewControlNavigationTargetIsModeled (control : ModelViewControl) : Bool := control.navigationType.isEmpty || ((control.navigationType == "modeled_view" || control.navigationType == "local_view_state") && control.navigationTarget.isEmpty == false) || (control.navigationType == "external_workflow" && control.externalWorkflow.isEmpty == false) || (control.navigationType == "external_system" && control.externalSystem.isEmpty == false && control.handoffContract.isEmpty == false)

def modelExternalBoundaryContractIsModeled (translation : ModelTranslationDefinition) : Bool := translation.translation.isEmpty == false && translation.externalEvent.isEmpty == false && translation.payloadContract.isEmpty == false && translation.command.isEmpty == false

def modelWorkflowBehaviorSurfaceIsComplete : Bool := modelOutcomes.all modelOutcomeBranchIsModeled && modelCommandErrors.all modelCommandErrorRecoveryIsModeled && modelViewControls.all modelViewControlNavigationTargetIsModeled && modelTranslationDefinitions.all modelExternalBoundaryContractIsModeled

theorem modelIdentityIsStable : modelName = "cfk-kernel" := rfl

theorem modelVersionIsStable : modelVersion = "0.1.0" := rfl

theorem modelDigestIsStable : modelDigest = "72fb303634e722acb3e503ae5a63a22cc9aa0ec51c297bcd45d6db6e2655f78b" := rfl

theorem modelWorkflowsAreDeclared : modelWorkflows.length = 22 := rfl

theorem modelSlicesAreDeclared : modelSlices.length = 64 := rfl

theorem modelSliceModulesAreDeclared : modelSliceModules.length = 64 := rfl

theorem modelWorkflowCompositionStructureComplete : (modelSlices.all modelSliceBelongsToDeclaredWorkflow && modelSlices.all modelSliceHasModule && modelSliceModules.all modelSliceModuleBelongsToDeclaredSlice && modelWorkflows.all modelWorkflowHasCompositionStructure) = true := rfl

theorem modelWorkflowBehaviorSurfaceIsCompleteIsStable : modelWorkflowBehaviorSurfaceIsComplete = true := rfl

theorem modelScenariosAreDeclared : modelScenarios.length = 0 := rfl

theorem modelScenarioDefinitionsAreDeclared : modelScenarioDefinitions.length = 0 := rfl

theorem modelScenarioDefinitionsHaveGwt : modelScenarioDefinitions.all modelScenarioDefinitionHasGwt = true := rfl

theorem modelScenarioKindsAreFirstClass : modelScenarioDefinitions.all modelScenarioKindIsFirstClass = true := rfl

theorem modelDataFlowsAreDeclared : modelDataFlows.length = 0 := rfl

theorem modelDataFlowsAreBitComplete : modelDataFlows.all modelDataFlowIsBitComplete = true := rfl

theorem modelDataFlowSourceKindsAreModeled : modelDataFlows.all modelDataFlowHasModeledSourceKind = true := rfl

theorem modelDataFlowModeledSourcesResolve : modelDataFlows.all modelDataFlowModeledSourceResolves = true := rfl

theorem modelDataFlowSourceChainsReachOriginals : modelDataFlows.all modelDataFlowHasOriginalSourceChain = true := rfl

theorem modelDataFlowSourceChainsPreserveBitEncodingSemantics : modelDataFlows.all modelDataFlowHasBitPreservingOriginalSourceChain = true := rfl

theorem modelDataFlowTransformationsAreModeled : modelDataFlows.all modelDataFlowHasModeledTransformationSemantics = true := rfl

theorem modelMeaningfulDataFlowsAreCovered : modelMeaningfulDataHasModeledDataFlows = true := rfl

theorem modelDataFlowSourceBitEncodingsMatchModeledSources : modelDataFlows.all modelDataFlowSourceBitEncodingMatchesModeledSource = true := rfl

theorem modelViewFieldBitEncodingsMatchDataFlows : modelViewFields.all modelViewFieldBitEncodingMatchesDataFlow = true := rfl

theorem modelExternalPayloadFieldBitEncodingsMatchDataFlows : modelExternalPayloadFields.all modelExternalPayloadFieldBitEncodingMatchesDataFlow = true := rfl

theorem modelOutcomesAreDeclared : modelOutcomes.length = 0 := rfl

theorem modelCommandErrorsAreDeclared : modelCommandErrors.length = 0 := rfl

theorem modelCommandsAreDeclared : modelCommands.length = 0 := rfl

theorem modelCommandInputsAreDeclared : modelCommandInputs.length = 0 := rfl

theorem modelCommandInputsHaveProvenance : modelCommandInputs.all modelCommandInputHasProvenance = true := rfl

theorem modelCommandInputsTraceToInvocationSources : modelCommandInputs.all modelCommandInputTracesToInvocationSource = true := rfl

theorem modelReadModelsAreDeclared : modelReadModels.length = 0 := rfl

theorem modelReadModelDefinitionsAreDeclared : modelReadModelDefinitions.length = 0 := rfl

theorem modelReadModelFieldsAreDeclared : modelReadModelFields.length = 0 := rfl

theorem modelEventAttributeSourcesAreComplete : modelEventAttributes.all modelEventAttributeSourceIsComplete = true := rfl

theorem modelReadModelFieldSourcesAreComplete : modelReadModelFields.all modelReadModelFieldSourceIsComplete = true := rfl

theorem modelViewFieldSourcesAreComplete : modelViewFields.all modelViewFieldSourceIsComplete = true := rfl

theorem modelViewFieldReadModelFieldSourcesResolve : modelViewFields.all modelViewFieldReadModelFieldSourceResolves = true := rfl

theorem modelDisplayedDataTraceToOriginalProvenance : modelViewFields.all modelDisplayedDatumTracesToOriginalProvenance = true := rfl

theorem modelExternalPayloadFieldsHaveProvenance : modelExternalPayloadFields.all modelExternalPayloadFieldHasProvenance = true := rfl

theorem modelViewsAreDeclared : modelViews.length = 0 := rfl

theorem modelViewDefinitionsAreDeclared : modelViewDefinitions.length = 0 := rfl

theorem modelViewControlsAreDeclared : modelViewControls.length = 0 := rfl

theorem modelViewControlsProvideCommandInputs : modelViewControls.all modelViewControlProvidesEveryCommandInput = true := rfl

theorem modelBoardElementsAreDeclared : modelBoardElements.length = 0 := rfl

theorem modelBoardConnectionsAreDeclared : modelBoardConnections.length = 0 := rfl

theorem modelViewFieldsAreDeclared : modelViewFields.length = 0 := rfl

theorem modelAutomationsAreDeclared : modelAutomations.length = 0 := rfl

theorem modelAutomationDefinitionsAreDeclared : modelAutomationDefinitions.length = 0 := rfl

theorem modelTranslationsAreDeclared : modelTranslations.length = 0 := rfl

theorem modelTranslationDefinitionsAreDeclared : modelTranslationDefinitions.length = 0 := rfl

theorem modelExternalPayloadsAreDeclared : modelExternalPayloads.length = 0 := rfl

theorem modelExternalPayloadFieldsAreDeclared : modelExternalPayloadFields.length = 0 := rfl

theorem modelStreamsAreDeclared : modelStreams.length = 0 := rfl

theorem modelEventsAreDeclared : modelEvents.length = 0 := rfl

theorem modelEventAttributesAreDeclared : modelEventAttributes.length = 0 := rfl

end CfkKernel
