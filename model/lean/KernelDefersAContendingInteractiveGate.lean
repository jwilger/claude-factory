namespace KernelDefersAContendingInteractiveGate

-- EMC-DIGEST: workflow:name=Kernel Defers a Contending Interactive Gate;slug=kernel-defers-contending-gate;description=Workflow covering gate contention in concurrent phase runs: when a second work item reaches an interactive gate while another gate lock is already held, the second gate is deferred and re-enqueued automatically when the first gate's lock is released.;slices=detect-gate-lock-contention|Detect Gate Lock Contention|state_view|Given a gate step is scheduled for work item B, When kernel checks current gate lock state and finds GateLockAcquired for a different work item A, Then GateContention outcome returned with blocking_work_item_id so the kernel knows it cannot open gate B yet|entry,emit-gate-deferred|Emit GateDeferred for Contending Work Item|state_change|Given GateContention detected for work item B, When kernel cannot open the gate immediately, Then GateDeferred event emitted with work_item_id, gate_kind, blocking_work_item_id, and deferred_at_ms; work item B moves to waiting state until the lock is released|main,reenqueue-deferred-gate-on-lock-release|Re-enqueue Deferred Gate When Lock Released|automation|Given GateLockReleased event emitted by work item A, When kernel scans for deferred gates, Then any work item with a GateDeferred record where blocking_work_item_id matches A is re-enqueued as ready so the next cf_next_step call will schedule it|main;transitions=detect-gate-lock-contention->emit-gate-deferred:outcome:GateContention::,emit-gate-deferred->reenqueue-deferred-gate-on-lock-release:event:GateDeferred::{ work_item_id: Uuid, gate_kind: String, blocking_work_item_id: Uuid, deferred_at_ms: u64 };outcomes=;command_errors=;owned_definitions=;transition_evidences=;entry_lifecycle_required=false;entry_lifecycle_states=
-- EMC generated Lean4 business workflow model.
def workflowName := "Kernel Defers a Contending Interactive Gate"

def workflowSlug := "kernel-defers-contending-gate"

def workflowDescription := "Workflow covering gate contention in concurrent phase runs: when a second work item reaches an interactive gate while another gate lock is already held, the second gate is deferred and re-enqueued automatically when the first gate's lock is released."

structure WorkflowSlice where
  slug : String

def workflowSlices : List WorkflowSlice := [{ slug := "detect-gate-lock-contention" },{ slug := "emit-gate-deferred" },{ slug := "reenqueue-deferred-gate-on-lock-release" }]

def workflowSliceSlugs : List String := workflowSlices.map (fun slice => slice.slug)

inductive SliceKindName where
  | stateView
  | stateChange
  | translation
  | automation
deriving BEq, DecidableEq, Repr

structure WorkflowSliceDetail where
  slug : String
  name : String
  kind : SliceKindName
  description : String

structure WorkflowSliceModule where
  slice : String
  formalModule : String

inductive WorkflowTransitionKind where
  | command
  | event
  | navigation
  | externalTrigger
  | outcome
  | workflowExitCommand
  | workflowExitEvent
  | workflowExitNavigation
  | workflowExitExternalTrigger
  | workflowExitOutcome
deriving BEq, DecidableEq, Repr

inductive WorkflowOwnedDefinitionKind where
  | command
  | event
  | view
  | control
  | readModel
  | outcome
  | error
  | automation
  | translation
  | externalPayload
deriving BEq, DecidableEq, Repr

inductive WorkflowStepRelationshipName where
  | entry
  | main
  | branch
  | alternate
  | asyncLifecycle
  | supporting
deriving BEq, DecidableEq, Repr

inductive WorkflowEntryLifecycleStateName where
  | freshUninitialized
  | initializedUnauthenticated
  | initializedAuthenticated
  | partiallyConfigured
  | fullyConfigured
deriving BEq, DecidableEq, Repr

def workflowSliceDetails : List WorkflowSliceDetail := [{ slug := "detect-gate-lock-contention", name := "Detect Gate Lock Contention", kind := SliceKindName.stateView, description := "Given a gate step is scheduled for work item B, When kernel checks current gate lock state and finds GateLockAcquired for a different work item A, Then GateContention outcome returned with blocking_work_item_id so the kernel knows it cannot open gate B yet" },{ slug := "emit-gate-deferred", name := "Emit GateDeferred for Contending Work Item", kind := SliceKindName.stateChange, description := "Given GateContention detected for work item B, When kernel cannot open the gate immediately, Then GateDeferred event emitted with work_item_id, gate_kind, blocking_work_item_id, and deferred_at_ms; work item B moves to waiting state until the lock is released" },{ slug := "reenqueue-deferred-gate-on-lock-release", name := "Re-enqueue Deferred Gate When Lock Released", kind := SliceKindName.automation, description := "Given GateLockReleased event emitted by work item A, When kernel scans for deferred gates, Then any work item with a GateDeferred record where blocking_work_item_id matches A is re-enqueued as ready so the next cf_next_step call will schedule it" }]

def workflowSliceModules : List WorkflowSliceModule := [{ slice := "detect-gate-lock-contention", formalModule := "DetectGateLockContention" },{ slice := "emit-gate-deferred", formalModule := "EmitGateDeferredForContendingWorkItem" },{ slice := "reenqueue-deferred-gate-on-lock-release", formalModule := "ReEnqueueDeferredGateWhenLockReleased" }]

structure WorkflowTransition where
  source : String
  target : String
  kind : WorkflowTransitionKind
  trigger : String
  sourceControl : String
  targetView : String
  rationale : String
  payloadContract : String

structure WorkflowOutcome where
  sourceSlice : String
  label : String
  externallyRelevant : Bool

structure WorkflowCommandError where
  sourceSlice : String
  commandName : String
  errorName : String

structure WorkflowOwnedDefinition where
  sourceSlice : String
  definitionKind : WorkflowOwnedDefinitionKind
  definitionName : String
  definitionStream : String
  sourceProvenance : String
  eventParticipation : String
  viewRole : String

structure WorkflowTransitionEvidence where
  source : String
  target : String
  kind : WorkflowTransitionKind
  trigger : String
  sourceControl : String
  targetView : String
  sourceEvidence : String
  targetEvidence : String

structure WorkflowEntryLifecycleState where
  state : WorkflowEntryLifecycleStateName
  step : String
  evidence : String

def workflowTransitions : List WorkflowTransition := [{ source := "detect-gate-lock-contention", target := "emit-gate-deferred", kind := WorkflowTransitionKind.outcome, trigger := "GateContention", sourceControl := "", targetView := "", rationale := "", payloadContract := "" },{ source := "emit-gate-deferred", target := "reenqueue-deferred-gate-on-lock-release", kind := WorkflowTransitionKind.event, trigger := "GateDeferred", sourceControl := "", targetView := "", rationale := "", payloadContract := "{ work_item_id: Uuid, gate_kind: String, blocking_work_item_id: Uuid, deferred_at_ms: u64 }" }]

def workflowOutcomes : List WorkflowOutcome := []

def workflowCommandErrors : List WorkflowCommandError := []

def workflowOwnedDefinitions : List WorkflowOwnedDefinition := []

def workflowTransitionEvidences : List WorkflowTransitionEvidence := []

def workflowRequiresEntryLifecycleCoverage : Bool := false

def workflowEntryLifecycleStates : List WorkflowEntryLifecycleState := []

def workflowExitTargets : List String := []

def requiredEntryLifecycleStates : List WorkflowEntryLifecycleStateName := [WorkflowEntryLifecycleStateName.freshUninitialized,WorkflowEntryLifecycleStateName.initializedUnauthenticated,WorkflowEntryLifecycleStateName.initializedAuthenticated,WorkflowEntryLifecycleStateName.partiallyConfigured,WorkflowEntryLifecycleStateName.fullyConfigured]

structure WorkflowStepRelationship where
  step : String
  relationship : WorkflowStepRelationshipName

def workflowStepRelationships : List WorkflowStepRelationship := [{ step := "detect-gate-lock-contention", relationship := WorkflowStepRelationshipName.entry },{ step := "emit-gate-deferred", relationship := WorkflowStepRelationshipName.main },{ step := "reenqueue-deferred-gate-on-lock-release", relationship := WorkflowStepRelationshipName.main }]

def workflowStepRelationshipIsAllowed (step : WorkflowStepRelationship) : Bool := workflowSliceSlugs.contains step.step

def workflowStepRelationshipsAreAllowed : Bool := workflowStepRelationships.all workflowStepRelationshipIsAllowed

def workflowStepSlugCount (slug : String) : Nat := (workflowSliceSlugs.filter (fun step => step == slug)).length

def workflowStepSlugsAreUnique : Bool := workflowSliceSlugs.all (fun step => workflowStepSlugCount step == 1)

def workflowEntryStepCount : Nat := (workflowStepRelationships.filter (fun step => step.relationship == WorkflowStepRelationshipName.entry)).length

def workflowHasExactlyOneEntryStep : Bool := workflowEntryStepCount == 1

def workflowMainStepHasIncomingTransition (step : WorkflowStepRelationship) : Bool := step.relationship != WorkflowStepRelationshipName.main || workflowTransitions.any (fun transition => transition.target == step.step)

def workflowMainStepsHaveIncomingReachability : Bool := workflowStepRelationships.all workflowMainStepHasIncomingTransition

def workflowEntrySteps : List String := (workflowStepRelationships.filter (fun step => step.relationship == WorkflowStepRelationshipName.entry)).map (fun step => step.step)

def workflowTargetsFromReachable (reachable : List String) : List String := (workflowTransitions.filter (fun transition => reachable.contains transition.source && workflowSliceSlugs.contains transition.target)).map (fun transition => transition.target)

def workflowReachableStepsAfterFuel : Nat -> List String -> List String
  | Nat.zero, reachable => reachable
  | Nat.succ fuel, reachable => workflowReachableStepsAfterFuel fuel (reachable ++ workflowTargetsFromReachable reachable)

def workflowReachableStepsFromEntry : List String := workflowReachableStepsAfterFuel workflowSlices.length workflowEntrySteps

def workflowStepIsReachableFromEntry (step : WorkflowStepRelationship) : Bool := step.relationship == WorkflowStepRelationshipName.supporting || step.relationship == WorkflowStepRelationshipName.asyncLifecycle || workflowReachableStepsFromEntry.contains step.step

def workflowNonSupportingStepsReachableFromEntry : Bool := workflowStepRelationships.all workflowStepIsReachableFromEntry

def workflowBranchOrAlternateStepHasTriggerOrRationale (step : WorkflowStepRelationship) : Bool := (step.relationship != WorkflowStepRelationshipName.branch && step.relationship != WorkflowStepRelationshipName.alternate) || workflowTransitions.any (fun transition => transition.target == step.step && (transition.trigger.isEmpty == false || transition.rationale.isEmpty == false))

def workflowBranchAndAlternateStepsHaveTriggerOrRationale : Bool := workflowStepRelationships.all workflowBranchOrAlternateStepHasTriggerOrRationale

def workflowTransitionKindIsModeled (transition : WorkflowTransition) : Bool := transition.kind == WorkflowTransitionKind.navigation || transition.kind == WorkflowTransitionKind.command || transition.kind == WorkflowTransitionKind.event || transition.kind == WorkflowTransitionKind.externalTrigger || transition.kind == WorkflowTransitionKind.outcome || transition.kind == WorkflowTransitionKind.workflowExitNavigation || transition.kind == WorkflowTransitionKind.workflowExitCommand || transition.kind == WorkflowTransitionKind.workflowExitEvent || transition.kind == WorkflowTransitionKind.workflowExitExternalTrigger || transition.kind == WorkflowTransitionKind.workflowExitOutcome

def workflowTransitionExitHasRationale (transition : WorkflowTransition) : Bool := workflowExitTargets.contains transition.target == false || transition.rationale.isEmpty == false

def workflowTransitionsHaveModeledKinds : Bool := workflowTransitions.all workflowTransitionKindIsModeled

def workflowExitsNameTargetsAndRationale : Bool := workflowTransitions.all workflowTransitionExitHasRationale

def workflowOutcomeHandledByTransition (outcome : WorkflowOutcome) : Bool := outcome.externallyRelevant == false || workflowTransitions.any (fun transition => transition.source == outcome.sourceSlice && transition.kind == WorkflowTransitionKind.outcome && transition.trigger == outcome.label)

def workflowExternallyRelevantOutcomesHandled : Bool := workflowOutcomes.all workflowOutcomeHandledByTransition

def workflowOutcomeSourceResolves (outcome : WorkflowOutcome) : Bool := workflowSliceSlugs.contains outcome.sourceSlice

def workflowOutcomesSourceResolve : Bool := workflowOutcomes.all workflowOutcomeSourceResolves

def workflowCommandErrorSourceResolves (error : WorkflowCommandError) : Bool := workflowSliceSlugs.contains error.sourceSlice

def workflowCommandErrorsSourceResolve : Bool := workflowCommandErrors.all workflowCommandErrorSourceResolves

def workflowTransitionIsNotCommandErrorOutcome (transition : WorkflowTransition) : Bool := transition.kind != WorkflowTransitionKind.outcome || workflowCommandErrors.any (fun error => error.sourceSlice == transition.source && error.errorName == transition.trigger) == false

def workflowTransitionsDoNotUseCommandErrorsAsOutcomes : Bool := workflowTransitions.all workflowTransitionIsNotCommandErrorOutcome

def workflowNonEventDefinitionOwnedOnce (definition : WorkflowOwnedDefinition) : Bool := definition.definitionKind == WorkflowOwnedDefinitionKind.event || (workflowOwnedDefinitions.filter (fun other => other.definitionKind == definition.definitionKind && other.definitionName == definition.definitionName)).length == 1

def workflowNonEventDefinitionsAreUniquelyOwned : Bool := workflowOwnedDefinitions.all workflowNonEventDefinitionOwnedOnce

def workflowEventDefinitionHasIdentity (definition : WorkflowOwnedDefinition) : Bool := definition.definitionKind != WorkflowOwnedDefinitionKind.event || (definition.definitionStream.isEmpty == false && definition.sourceProvenance.isEmpty == false)

def workflowSharedEventDefinitionMatches (left : WorkflowOwnedDefinition) (right : WorkflowOwnedDefinition) : Bool := left.definitionKind != WorkflowOwnedDefinitionKind.event || right.definitionKind != WorkflowOwnedDefinitionKind.event || left.definitionName != right.definitionName || (left.definitionStream == right.definitionStream && left.sourceProvenance == right.sourceProvenance)

def workflowSharedEventDefinitionsHaveIdenticalIdentity : Bool := workflowOwnedDefinitions.all workflowEventDefinitionHasIdentity && workflowOwnedDefinitions.all (fun definition => workflowOwnedDefinitions.all (workflowSharedEventDefinitionMatches definition))

def workflowOnlyEventsMayBeSharedAcrossSlices : Bool := workflowNonEventDefinitionsAreUniquelyOwned && workflowSharedEventDefinitionsHaveIdenticalIdentity

def workflowOwnsDefinition (sourceSlice : String) (definitionKind : WorkflowOwnedDefinitionKind) (definitionName : String) : Bool := workflowOwnedDefinitions.any (fun definition => definition.sourceSlice == sourceSlice && definition.definitionKind == definitionKind && definition.definitionName == definitionName)

def workflowSliceHasKind (slice : String) (kind : SliceKindName) : Bool := workflowSliceDetails.any (fun detail => detail.slug == slice && detail.kind == kind)

def workflowEventParticipationIsModeled (definition : WorkflowOwnedDefinition) : Bool := definition.eventParticipation == "emitted" || definition.eventParticipation == "observed"

def workflowEventDefinitionParticipates (sourceSlice : String) (eventName : String) : Bool := workflowOwnedDefinitions.any (fun definition => definition.sourceSlice == sourceSlice && definition.definitionKind == WorkflowOwnedDefinitionKind.event && definition.definitionName == eventName && workflowEventParticipationIsModeled definition)

def workflowViewRoleIsEntry (definition : WorkflowOwnedDefinition) : Bool := definition.viewRole == "entry"

def workflowOwnsEntryView (sourceSlice : String) (viewName : String) : Bool := workflowOwnedDefinitions.any (fun definition => definition.sourceSlice == sourceSlice && definition.definitionKind == WorkflowOwnedDefinitionKind.view && definition.definitionName == viewName && workflowViewRoleIsEntry definition)

def workflowNavigationSourceControl (transition : WorkflowTransition) : String := transition.sourceControl

def workflowNavigationTargetView (transition : WorkflowTransition) : String := transition.targetView

def workflowCommandTransitionTargetsOwnedCommand (transition : WorkflowTransition) : Bool := transition.kind != WorkflowTransitionKind.command || workflowOwnsDefinition transition.target WorkflowOwnedDefinitionKind.command transition.trigger

def workflowCommandTransitionsTargetOwnedCommands : Bool := workflowTransitions.all workflowCommandTransitionTargetsOwnedCommand

def workflowCommandTransitionSourceOwnsControl (transition : WorkflowTransition) : Bool := transition.kind != WorkflowTransitionKind.command || workflowOwnsDefinition transition.source WorkflowOwnedDefinitionKind.control transition.trigger

def workflowCommandTransitionsSourceOwnedControls : Bool := workflowTransitions.all workflowCommandTransitionSourceOwnsControl

def workflowCommandTransitionsResolveControlsAndCommands : Bool := workflowTransitions.all (fun transition => workflowCommandTransitionSourceOwnsControl transition && workflowCommandTransitionTargetsOwnedCommand transition)

def workflowStateViewCommandTransitionTargetsStateChange (transition : WorkflowTransition) : Bool := transition.kind != WorkflowTransitionKind.command || workflowSliceHasKind transition.source SliceKindName.stateView == false || workflowSliceHasKind transition.target SliceKindName.stateChange

def workflowStateViewCommandTransitionsTargetStateChanges : Bool := workflowTransitions.all workflowStateViewCommandTransitionTargetsStateChange

def workflowEventTransitionIsSharedByEndpoints (transition : WorkflowTransition) : Bool := transition.kind != WorkflowTransitionKind.event || (workflowOwnsDefinition transition.source WorkflowOwnedDefinitionKind.event transition.trigger && workflowOwnsDefinition transition.target WorkflowOwnedDefinitionKind.event transition.trigger)

def workflowEventTransitionsAreSharedByEndpointSlices : Bool := workflowTransitions.all workflowEventTransitionIsSharedByEndpoints

def workflowEventTransitionSourceParticipates (transition : WorkflowTransition) : Bool := transition.kind != WorkflowTransitionKind.event || workflowEventDefinitionParticipates transition.source transition.trigger

def workflowEventTransitionTargetParticipates (transition : WorkflowTransition) : Bool := transition.kind != WorkflowTransitionKind.event || workflowEventDefinitionParticipates transition.target transition.trigger

def workflowEventTransitionsHaveParticipatingEndpointEvents : Bool := workflowTransitions.all (fun transition => workflowEventTransitionSourceParticipates transition && workflowEventTransitionTargetParticipates transition)

def workflowNavigationTransitionSourceOwnsControl (transition : WorkflowTransition) : Bool := transition.kind != WorkflowTransitionKind.navigation || workflowOwnsDefinition transition.source WorkflowOwnedDefinitionKind.control (workflowNavigationSourceControl transition)

def workflowNavigationTransitionTargetsOwnedView (transition : WorkflowTransition) : Bool := transition.kind != WorkflowTransitionKind.navigation || workflowOwnsDefinition transition.target WorkflowOwnedDefinitionKind.view (workflowNavigationTargetView transition)

def workflowNavigationTransitionTargetsEntryView (transition : WorkflowTransition) : Bool := transition.kind != WorkflowTransitionKind.navigation || workflowOwnsEntryView transition.target (workflowNavigationTargetView transition)

def workflowNavigationTransitionsResolveControlsAndViews : Bool := workflowTransitions.all (fun transition => workflowNavigationTransitionSourceOwnsControl transition && workflowNavigationTransitionTargetsOwnedView transition)

def workflowNavigationTransitionsResolveToEntryViews : Bool := workflowTransitions.all workflowNavigationTransitionTargetsEntryView

def workflowExternalTriggerDeclaresPayloadContract (transition : WorkflowTransition) : Bool := transition.kind != WorkflowTransitionKind.externalTrigger || (transition.payloadContract.isEmpty == false && workflowOwnsDefinition transition.source WorkflowOwnedDefinitionKind.externalPayload transition.payloadContract)

def workflowExternalTriggersDeclarePayloadContracts : Bool := workflowTransitions.all workflowExternalTriggerDeclaresPayloadContract

def workflowExternalTriggerPayloadContractHasProvenance (transition : WorkflowTransition) : Bool := transition.kind != WorkflowTransitionKind.externalTrigger || workflowOwnedDefinitions.any (fun definition => definition.sourceSlice == transition.source && definition.definitionKind == WorkflowOwnedDefinitionKind.externalPayload && definition.definitionName == transition.payloadContract && definition.sourceProvenance.isEmpty == false)

def workflowExternalTriggerPayloadContractsHaveProvenance : Bool := workflowTransitions.all workflowExternalTriggerPayloadContractHasProvenance

def workflowTransitionRequiresEvidence (transition : WorkflowTransition) : Bool := transition.kind == WorkflowTransitionKind.event || transition.kind == WorkflowTransitionKind.command || transition.kind == WorkflowTransitionKind.navigation

def workflowTransitionEvidenceMatches (transition : WorkflowTransition) (evidence : WorkflowTransitionEvidence) : Bool := evidence.source == transition.source && evidence.target == transition.target && evidence.kind == transition.kind && evidence.trigger == transition.trigger && (transition.kind != WorkflowTransitionKind.navigation || ((evidence.sourceControl.isEmpty || evidence.sourceControl == workflowNavigationSourceControl transition) && (evidence.targetView.isEmpty || evidence.targetView == workflowNavigationTargetView transition))) && evidence.sourceEvidence.isEmpty == false && evidence.targetEvidence.isEmpty == false

def workflowTransitionHasRequiredEvidence (transition : WorkflowTransition) : Bool := workflowTransitionRequiresEvidence transition == false || workflowTransitionEvidences.any (workflowTransitionEvidenceMatches transition)

def workflowTransitionsHaveRequiredEvidence : Bool := workflowTransitions.all workflowTransitionHasRequiredEvidence

def workflowEntryLifecycleStateCovered (state : WorkflowEntryLifecycleStateName) : Bool := workflowEntryLifecycleStates.any (fun coverage => coverage.state == state && workflowSliceSlugs.contains coverage.step && coverage.evidence.isEmpty == false)

def workflowEntryLifecycleStatesCoverRequiredStates : Bool := workflowRequiresEntryLifecycleCoverage == false || requiredEntryLifecycleStates.all workflowEntryLifecycleStateCovered

theorem workflowIdentityIsStable : workflowName = "Kernel Defers a Contending Interactive Gate" := rfl

theorem workflowSlicesHaveDetails : workflowSlices.length = workflowSliceDetails.length := rfl

theorem workflowSlicesHaveModuleReferences : workflowSlices.length = workflowSliceModules.length := rfl

theorem workflowTransitionsAreStructured : workflowTransitions.all (fun transition => transition.source.isEmpty == false && transition.target.isEmpty == false && transition.trigger.isEmpty == false) = true := by native_decide

theorem workflowTransitionSourcesResolve : workflowTransitions.all (fun transition => workflowSliceSlugs.contains transition.source) = true := by native_decide

theorem workflowTransitionTargetsResolve : workflowTransitions.all (fun transition => workflowSliceSlugs.contains transition.target || workflowExitTargets.contains transition.target) = true := by native_decide

theorem workflowStepRelationshipsAreAllowedIsStable : workflowStepRelationshipsAreAllowed = true := by native_decide

theorem workflowStepSlugsAreUniqueIsStable : workflowStepSlugsAreUnique = true := by native_decide

theorem workflowHasExactlyOneEntryStepIsStable : workflowHasExactlyOneEntryStep = true := by native_decide

theorem workflowMainStepsHaveIncomingReachabilityIsStable : workflowMainStepsHaveIncomingReachability = true := by native_decide

theorem workflowNonSupportingStepsReachableFromEntryIsStable : workflowNonSupportingStepsReachableFromEntry = true := by native_decide

theorem workflowBranchAndAlternateStepsHaveTriggerOrRationaleIsStable : workflowBranchAndAlternateStepsHaveTriggerOrRationale = true := by native_decide

theorem workflowTransitionsHaveModeledKindsIsStable : workflowTransitionsHaveModeledKinds = true := by native_decide

theorem workflowExitsNameTargetsAndRationaleIsStable : workflowExitsNameTargetsAndRationale = true := by native_decide

theorem workflowExternallyRelevantOutcomesHandledIsStable : workflowExternallyRelevantOutcomesHandled = true := by native_decide

theorem workflowOutcomesSourceResolveIsStable : workflowOutcomesSourceResolve = true := by native_decide

theorem workflowCommandErrorsSourceResolveIsStable : workflowCommandErrorsSourceResolve = true := by native_decide

theorem workflowTransitionsDoNotUseCommandErrorsAsOutcomesIsStable : workflowTransitionsDoNotUseCommandErrorsAsOutcomes = true := by native_decide

theorem workflowNonEventDefinitionsAreUniquelyOwnedIsStable : workflowNonEventDefinitionsAreUniquelyOwned = true := by native_decide

theorem workflowSharedEventDefinitionsHaveIdenticalIdentityIsStable : workflowSharedEventDefinitionsHaveIdenticalIdentity = true := by native_decide

theorem workflowOnlyEventsMayBeSharedAcrossSlicesIsStable : workflowOnlyEventsMayBeSharedAcrossSlices = true := by native_decide

theorem workflowCommandTransitionsTargetOwnedCommandsIsStable : workflowCommandTransitionsTargetOwnedCommands = true := by native_decide

theorem workflowCommandTransitionsSourceOwnedControlsIsStable : workflowCommandTransitionsSourceOwnedControls = true := by native_decide

theorem workflowCommandTransitionsResolveControlsAndCommandsIsStable : workflowCommandTransitionsResolveControlsAndCommands = true := by native_decide

theorem workflowStateViewCommandTransitionsTargetStateChangesIsStable : workflowStateViewCommandTransitionsTargetStateChanges = true := by native_decide

theorem workflowEventTransitionsAreSharedByEndpointSlicesIsStable : workflowEventTransitionsAreSharedByEndpointSlices = true := by native_decide

theorem workflowEventTransitionsHaveParticipatingEndpointEventsIsStable : workflowEventTransitionsHaveParticipatingEndpointEvents = true := by native_decide

theorem workflowNavigationTransitionsResolveControlsAndViewsIsStable : workflowNavigationTransitionsResolveControlsAndViews = true := by native_decide

theorem workflowNavigationTransitionsResolveToEntryViewsIsStable : workflowNavigationTransitionsResolveToEntryViews = true := by native_decide

theorem workflowExternalTriggersDeclarePayloadContractsIsStable : workflowExternalTriggersDeclarePayloadContracts = true := by native_decide

theorem workflowExternalTriggerPayloadContractsHaveProvenanceIsStable : workflowExternalTriggerPayloadContractsHaveProvenance = true := by native_decide

theorem workflowTransitionsHaveRequiredEvidenceIsStable : workflowTransitionsHaveRequiredEvidence = true := by native_decide

theorem workflowEntryLifecycleStatesCoverRequiredStatesIsStable : workflowEntryLifecycleStatesCoverRequiredStates = true := by native_decide

end KernelDefersAContendingInteractiveGate
