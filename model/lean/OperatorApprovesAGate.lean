namespace OperatorApprovesAGate

-- EMC-DIGEST: workflow:name=Operator Approves a Gate;slug=operator-approves-gate;description=Workflow covering the gate approval path: a reviewer agent or human operator returns an Approved verdict via cf_gate, the kernel emits GateApproved, and the work item advances to Done with WorkItemCompleted emitted.;slices=reviewer-submits-approval|Reviewer Submits Approval via cf_gate|state_change|Given gate_review step completed and reviewer returns Approved verdict, When conductor calls cf_gate with verdict=approved and reviewer_id, Then GateApproved event emitted with work_item_id, gate_kind, reviewer_id, and approved_at_ms|entry,advance-work-item-to-done-on-approval|Advance Work Item to Done on Approval|automation|Given GateApproved event emitted, When the gate is the final gate for this work item's phase, Then WorkItemCompleted event emitted and work item transitions to Done state; done count for the phase increments in cf_status|main,expose-completed-item-in-status|Expose Completed Work Item in cf_status|state_view|Given WorkItemCompleted event emitted, When cf_status is called, Then phase row shows the done count incremented by 1 and in_progress count decremented by 1; completed item no longer appears in active backlog|main;transitions=reviewer-submits-approval->advance-work-item-to-done-on-approval:event:GateApproved::{ work_item_id: Uuid, gate_kind: String, reviewer_id: String, approved_at_ms: u64 },advance-work-item-to-done-on-approval->expose-completed-item-in-status:event:WorkItemCompleted::{ work_item_id: Uuid, completed_at_ms: u64 };outcomes=;command_errors=;owned_definitions=;transition_evidences=;entry_lifecycle_required=false;entry_lifecycle_states=
-- EMC generated Lean4 business workflow model.
def workflowName := "Operator Approves a Gate"

def workflowSlug := "operator-approves-gate"

def workflowDescription := "Workflow covering the gate approval path: a reviewer agent or human operator returns an Approved verdict via cf_gate, the kernel emits GateApproved, and the work item advances to Done with WorkItemCompleted emitted."

structure WorkflowSlice where
  slug : String

def workflowSlices : List WorkflowSlice := [{ slug := "reviewer-submits-approval" },{ slug := "advance-work-item-to-done-on-approval" },{ slug := "expose-completed-item-in-status" }]

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

def workflowSliceDetails : List WorkflowSliceDetail := [{ slug := "reviewer-submits-approval", name := "Reviewer Submits Approval via cf_gate", kind := SliceKindName.stateChange, description := "Given gate_review step completed and reviewer returns Approved verdict, When conductor calls cf_gate with verdict=approved and reviewer_id, Then GateApproved event emitted with work_item_id, gate_kind, reviewer_id, and approved_at_ms" },{ slug := "advance-work-item-to-done-on-approval", name := "Advance Work Item to Done on Approval", kind := SliceKindName.automation, description := "Given GateApproved event emitted, When the gate is the final gate for this work item's phase, Then WorkItemCompleted event emitted and work item transitions to Done state; done count for the phase increments in cf_status" },{ slug := "expose-completed-item-in-status", name := "Expose Completed Work Item in cf_status", kind := SliceKindName.stateView, description := "Given WorkItemCompleted event emitted, When cf_status is called, Then phase row shows the done count incremented by 1 and in_progress count decremented by 1; completed item no longer appears in active backlog" }]

def workflowSliceModules : List WorkflowSliceModule := [{ slice := "reviewer-submits-approval", formalModule := "ReviewerSubmitsApprovalViaCfGate" },{ slice := "advance-work-item-to-done-on-approval", formalModule := "AdvanceWorkItemToDoneOnApproval" },{ slice := "expose-completed-item-in-status", formalModule := "ExposeCompletedWorkItemInCfStatus" }]

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

def workflowTransitions : List WorkflowTransition := [{ source := "reviewer-submits-approval", target := "advance-work-item-to-done-on-approval", kind := WorkflowTransitionKind.event, trigger := "GateApproved", sourceControl := "", targetView := "", rationale := "", payloadContract := "{ work_item_id: Uuid, gate_kind: String, reviewer_id: String, approved_at_ms: u64 }" },{ source := "advance-work-item-to-done-on-approval", target := "expose-completed-item-in-status", kind := WorkflowTransitionKind.event, trigger := "WorkItemCompleted", sourceControl := "", targetView := "", rationale := "", payloadContract := "{ work_item_id: Uuid, completed_at_ms: u64 }" }]

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

def workflowStepRelationships : List WorkflowStepRelationship := [{ step := "reviewer-submits-approval", relationship := WorkflowStepRelationshipName.entry },{ step := "advance-work-item-to-done-on-approval", relationship := WorkflowStepRelationshipName.main },{ step := "expose-completed-item-in-status", relationship := WorkflowStepRelationshipName.main }]

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

theorem workflowIdentityIsStable : workflowName = "Operator Approves a Gate" := rfl

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

end OperatorApprovesAGate
