module ProvisionWorktreeOnCreation {
  // EMC-DIGEST: workflow:name=Provision Worktree on Creation;slug=provision-worktree-on-creation;description=Workflow covering automatic git worktree provisioning when a development work item is claimed: the kernel runs git worktree add, emits WorktreeProvisioned, and makes the worktree_path available in the step context so the spawned agent writes code in an isolated directory.;slices=emit-provision-worktree-command|Emit ProvisionWorktree Command on Claim|automation|Given development work item claimed and worktree config is valid, When cf_claim succeeds, Then ProvisionWorktree command emitted with work_item_id, base_branch, and worktree_path derived from naming_template|entry,create-git-worktree|Create Git Worktree|state_change|Given ProvisionWorktree command received, When git worktree add is executed with the derived branch name and worktree_path, Then WorktreeProvisioned event emitted with work_item_id, worktree_path, branch_name, and provisioned_at_ms|main,expose-worktree-path-in-step-context|Expose Worktree Path in Step Context|state_view|Given WorktreeProvisioned event emitted for a work item, When cf_next_step returns the next step for that work item, Then step context includes worktree_path so the spawned agent writes code in the isolated worktree directory|main;transitions=emit-provision-worktree-command->create-git-worktree:command:ProvisionWorktree::,create-git-worktree->expose-worktree-path-in-step-context:event:WorktreeProvisioned::{ work_item_id: Uuid, worktree_path: PathBuf, branch_name: String, provisioned_at_ms: u64 };outcomes=;command_errors=;owned_definitions=;transition_evidences=;entry_lifecycle_required=false;entry_lifecycle_states=
  type WorkflowSlice = { slug: str }
  type SliceKindName = SliceStateView | SliceStateChange | SliceTranslation | SliceAutomation
  type WorkflowSliceDetail = { slug: str, name: str, kind: SliceKindName, description: str }
  type WorkflowSliceModule = { slice: str, formalModule: str }
  type WorkflowStepRelationshipName = StepEntry | StepMain | StepBranch | StepAlternate | StepAsyncLifecycle | StepSupporting
  type WorkflowStepRelationship = { step: str, relationship: WorkflowStepRelationshipName }
  type WorkflowEntryLifecycleStateName = FreshUninitialized | InitializedUnauthenticated | InitializedAuthenticated | PartiallyConfigured | FullyConfigured
  type WorkflowTransitionKind = Command | Event | Navigation | ExternalTrigger | Outcome | WorkflowExitCommand | WorkflowExitEvent | WorkflowExitNavigation | WorkflowExitExternalTrigger | WorkflowExitOutcome
  type WorkflowOwnedDefinitionKind = OwnedCommand | OwnedEvent | OwnedView | OwnedControl | OwnedReadModel | OwnedOutcome | OwnedError | OwnedAutomation | OwnedTranslation | OwnedExternalPayload
  type WorkflowTransition = { source: str, target: str, kind: WorkflowTransitionKind, trigger: str, sourceControl: str, targetView: str, rationale: str, payloadContract: str }
  type WorkflowOutcome = { sourceSlice: str, label: str, externallyRelevant: bool }
  type WorkflowCommandError = { sourceSlice: str, commandName: str, errorName: str }
  type WorkflowOwnedDefinition = { sourceSlice: str, definitionKind: WorkflowOwnedDefinitionKind, definitionName: str, definitionStream: str, sourceProvenance: str, eventParticipation: str, viewRole: str }
  type WorkflowTransitionEvidence = { source: str, target: str, kind: WorkflowTransitionKind, trigger: str, sourceControl: str, targetView: str, sourceEvidence: str, targetEvidence: str }
  type WorkflowEntryLifecycleState = { state: WorkflowEntryLifecycleStateName, step: str, evidence: str }
  val workflowName = "Provision Worktree on Creation"
  val workflowSlug = "provision-worktree-on-creation"
  val workflowDescription = "Workflow covering automatic git worktree provisioning when a development work item is claimed: the kernel runs git worktree add, emits WorktreeProvisioned, and makes the worktree_path available in the step context so the spawned agent writes code in an isolated directory."
  val workflowSlices: List[WorkflowSlice] = [{ slug: "emit-provision-worktree-command" },{ slug: "create-git-worktree" },{ slug: "expose-worktree-path-in-step-context" }]
  val workflowSliceSlugs: List[str] = workflowSlices.foldl([], (slugs, workflowSlice) => slugs.append(workflowSlice.slug))
  val workflowSliceDetails: List[WorkflowSliceDetail] = [{ slug: "emit-provision-worktree-command", name: "Emit ProvisionWorktree Command on Claim", kind: SliceAutomation, description: "Given development work item claimed and worktree config is valid, When cf_claim succeeds, Then ProvisionWorktree command emitted with work_item_id, base_branch, and worktree_path derived from naming_template" },{ slug: "create-git-worktree", name: "Create Git Worktree", kind: SliceStateChange, description: "Given ProvisionWorktree command received, When git worktree add is executed with the derived branch name and worktree_path, Then WorktreeProvisioned event emitted with work_item_id, worktree_path, branch_name, and provisioned_at_ms" },{ slug: "expose-worktree-path-in-step-context", name: "Expose Worktree Path in Step Context", kind: SliceStateView, description: "Given WorktreeProvisioned event emitted for a work item, When cf_next_step returns the next step for that work item, Then step context includes worktree_path so the spawned agent writes code in the isolated worktree directory" }]
  val workflowSliceModules: List[WorkflowSliceModule] = [{ slice: "emit-provision-worktree-command", formalModule: "EmitProvisionWorktreeCommandOnClaim" },{ slice: "create-git-worktree", formalModule: "CreateGitWorktree" },{ slice: "expose-worktree-path-in-step-context", formalModule: "ExposeWorktreePathInStepContext" }]
  val workflowStepRelationships: List[WorkflowStepRelationship] = [{ step: "emit-provision-worktree-command", relationship: StepEntry },{ step: "create-git-worktree", relationship: StepMain },{ step: "expose-worktree-path-in-step-context", relationship: StepMain }]
  val workflowTransitions: List[WorkflowTransition] = [{ source: "emit-provision-worktree-command", target: "create-git-worktree", kind: Command, trigger: "ProvisionWorktree", sourceControl: "", targetView: "", rationale: "", payloadContract: "" },{ source: "create-git-worktree", target: "expose-worktree-path-in-step-context", kind: Event, trigger: "WorktreeProvisioned", sourceControl: "", targetView: "", rationale: "", payloadContract: "{ work_item_id: Uuid, worktree_path: PathBuf, branch_name: String, provisioned_at_ms: u64 }" }]
  val workflowOutcomes: List[WorkflowOutcome] = []
  val workflowCommandErrors: List[WorkflowCommandError] = []
  val workflowOwnedDefinitions: List[WorkflowOwnedDefinition] = []
  val workflowTransitionEvidences: List[WorkflowTransitionEvidence] = []
  val workflowRequiresEntryLifecycleCoverage = false
  val workflowEntryLifecycleStates: List[WorkflowEntryLifecycleState] = []
  val workflowExitTargets: List[str] = []
  val requiredEntryLifecycleStates: List[WorkflowEntryLifecycleStateName] = [FreshUninitialized,InitializedUnauthenticated,InitializedAuthenticated,PartiallyConfigured,FullyConfigured]
  val workflowIdentityStable = workflowName == "Provision Worktree on Creation"
  val workflowSlicesHaveDetails = length(workflowSlices) == length(workflowSliceDetails)
  val workflowSliceDetailsComplete = workflowSlicesHaveDetails
  val workflowSliceModulesComplete = workflowSlices.length() == workflowSliceModules.length()
  val workflowTransitionsStructured = workflowTransitions.select(transition => transition.source != "" and transition.target != "" and transition.trigger != "").length() == workflowTransitions.length()
  val workflowTransitionSourcesResolve = workflowTransitions.select(transition => workflowSliceSlugs.select(step => step == transition.source).length() > 0).length() == workflowTransitions.length()
  val workflowTransitionTargetsResolve = workflowTransitions.select(transition => workflowSliceSlugs.select(step => step == transition.target).length() > 0 or workflowExitTargets.select(exitTarget => exitTarget == transition.target).length() > 0).length() == workflowTransitions.length()
  def workflowStepRelationshipIsAllowed(step) = workflowSliceSlugs.select(workflowSlice => workflowSlice == step.step).length() > 0
  val workflowStepRelationshipsAreAllowed = workflowStepRelationships.select(step => workflowStepRelationshipIsAllowed(step)).length() == workflowStepRelationships.length()
  val workflowStepSlugsAreUnique = workflowSliceSlugs.select(step => workflowSliceSlugs.select(other => other == step).length() == 1).length() == workflowSliceSlugs.length()
  val workflowHasExactlyOneEntryStep = workflowStepRelationships.select(step => step.relationship == StepEntry).length() == 1
  def workflowMainStepHasIncomingTransition(step) = step.relationship != StepMain or workflowTransitions.select(transition => transition.target == step.step).length() > 0
  val workflowMainStepsHaveIncomingReachability = workflowStepRelationships.select(step => workflowMainStepHasIncomingTransition(step)).length() == workflowStepRelationships.length()
  val workflowEntrySteps: List[str] = workflowStepRelationships.select(step => step.relationship == StepEntry).foldl([], (entrySteps, step) => entrySteps.append(step.step))
  def workflowTargetsFromReachable(reachable) = workflowTransitions.select(transition => reachable.select(step => step == transition.source).length() > 0 and workflowSliceSlugs.select(step => step == transition.target).length() > 0).foldl([], (targets, transition) => targets.append(transition.target))
  def workflowReachableStepsAfterFuel(fuel, reachable) = range(0, fuel).foldl(reachable, (currentReachable, _) => currentReachable.concat(workflowTargetsFromReachable(currentReachable)))
  val workflowReachableStepsFromEntry = workflowReachableStepsAfterFuel(3, workflowEntrySteps)
  def workflowStepIsReachableFromEntry(step) = step.relationship == StepSupporting or step.relationship == StepAsyncLifecycle or workflowReachableStepsFromEntry.select(reachableStep => reachableStep == step.step).length() > 0
  val workflowNonSupportingStepsReachableFromEntry = workflowStepRelationships.select(step => workflowStepIsReachableFromEntry(step)).length() == workflowStepRelationships.length()
  def workflowBranchOrAlternateStepHasTriggerOrRationale(step) = (step.relationship != StepBranch and step.relationship != StepAlternate) or workflowTransitions.select(transition => transition.target == step.step and (transition.trigger != "" or transition.rationale != "")).length() > 0
  val workflowBranchAndAlternateStepsHaveTriggerOrRationale = workflowStepRelationships.select(step => workflowBranchOrAlternateStepHasTriggerOrRationale(step)).length() == workflowStepRelationships.length()
  def workflowTransitionKindIsModeled(transition) = transition.kind == Navigation or transition.kind == Command or transition.kind == Event or transition.kind == ExternalTrigger or transition.kind == Outcome or transition.kind == WorkflowExitNavigation or transition.kind == WorkflowExitCommand or transition.kind == WorkflowExitEvent or transition.kind == WorkflowExitExternalTrigger or transition.kind == WorkflowExitOutcome
  def workflowTransitionExitHasRationale(transition) = workflowExitTargets.select(exitTarget => exitTarget == transition.target).length() == 0 or transition.rationale != ""
  val workflowTransitionsHaveModeledKinds = workflowTransitions.select(transition => workflowTransitionKindIsModeled(transition)).length() == workflowTransitions.length()
  val workflowExitsNameTargetsAndRationale = workflowTransitions.select(transition => workflowTransitionExitHasRationale(transition)).length() == workflowTransitions.length()
  def workflowOutcomeHandledByTransition(outcome) = not(outcome.externallyRelevant) or workflowTransitions.select(transition => transition.source == outcome.sourceSlice and transition.kind == Outcome and transition.trigger == outcome.label).length() > 0
  val workflowExternallyRelevantOutcomesHandled = workflowOutcomes.select(outcome => workflowOutcomeHandledByTransition(outcome)).length() == workflowOutcomes.length()
  def workflowOutcomeSourceResolves(outcome) = workflowSliceSlugs.select(step => step == outcome.sourceSlice).length() > 0
  val workflowOutcomesSourceResolve = workflowOutcomes.select(outcome => workflowOutcomeSourceResolves(outcome)).length() == workflowOutcomes.length()
  def workflowCommandErrorSourceResolves(error) = workflowSliceSlugs.select(step => step == error.sourceSlice).length() > 0
  val workflowCommandErrorsSourceResolve = workflowCommandErrors.select(error => workflowCommandErrorSourceResolves(error)).length() == workflowCommandErrors.length()
  val workflowTransitionsDoNotUseCommandErrorsAsOutcomes = workflowTransitions.select(transition => transition.kind != Outcome or workflowCommandErrors.select(error => error.sourceSlice == transition.source and error.errorName == transition.trigger).length() == 0).length() == workflowTransitions.length()
  def workflowNonEventDefinitionOwnedOnce(definition) = definition.definitionKind == OwnedEvent or workflowOwnedDefinitions.select(other => other.definitionKind == definition.definitionKind and other.definitionName == definition.definitionName).length() == 1
  val workflowNonEventDefinitionsAreUniquelyOwned = workflowOwnedDefinitions.select(definition => workflowNonEventDefinitionOwnedOnce(definition)).length() == workflowOwnedDefinitions.length()
  def workflowEventDefinitionHasIdentity(definition) = definition.definitionKind != OwnedEvent or (definition.definitionStream != "" and definition.sourceProvenance != "")
  def workflowSharedEventDefinitionMatches(left, right) = left.definitionKind != OwnedEvent or right.definitionKind != OwnedEvent or left.definitionName != right.definitionName or (left.definitionStream == right.definitionStream and left.sourceProvenance == right.sourceProvenance)
  val workflowSharedEventDefinitionsHaveIdenticalIdentity = workflowOwnedDefinitions.select(definition => workflowEventDefinitionHasIdentity(definition)).length() == workflowOwnedDefinitions.length() and workflowOwnedDefinitions.select(definition => workflowOwnedDefinitions.select(other => workflowSharedEventDefinitionMatches(definition, other)).length() == workflowOwnedDefinitions.length()).length() == workflowOwnedDefinitions.length()
  val workflowOnlyEventsMayBeSharedAcrossSlices = workflowNonEventDefinitionsAreUniquelyOwned and workflowSharedEventDefinitionsHaveIdenticalIdentity
  def workflowOwnsDefinition(sourceSlice, definitionKind, definitionName) = workflowOwnedDefinitions.select(definition => definition.sourceSlice == sourceSlice and definition.definitionKind == definitionKind and definition.definitionName == definitionName).length() > 0
  def workflowEventParticipationIsModeled(definition) = definition.eventParticipation == "emitted" or definition.eventParticipation == "observed"
  def workflowEventDefinitionParticipates(sourceSlice, eventName) = workflowOwnedDefinitions.select(definition => definition.sourceSlice == sourceSlice and definition.definitionKind == OwnedEvent and definition.definitionName == eventName and workflowEventParticipationIsModeled(definition)).length() > 0
  def workflowViewRoleIsEntry(definition) = definition.viewRole == "entry"
  def workflowOwnsEntryView(sourceSlice, viewName) = workflowOwnedDefinitions.select(definition => definition.sourceSlice == sourceSlice and definition.definitionKind == OwnedView and definition.definitionName == viewName and workflowViewRoleIsEntry(definition)).length() > 0
  def workflowNavigationSourceControl(transition) = transition.sourceControl
  def workflowNavigationTargetView(transition) = transition.targetView
  def workflowCommandTransitionTargetsOwnedCommand(transition) = transition.kind != Command or workflowOwnsDefinition(transition.target, OwnedCommand, transition.trigger)
  val workflowCommandTransitionsTargetOwnedCommands = workflowTransitions.select(transition => workflowCommandTransitionTargetsOwnedCommand(transition)).length() == workflowTransitions.length()
  def workflowCommandTransitionSourceOwnsControl(transition) = transition.kind != Command or workflowOwnsDefinition(transition.source, OwnedControl, transition.trigger)
  val workflowCommandTransitionsSourceOwnedControls = workflowTransitions.select(transition => workflowCommandTransitionSourceOwnsControl(transition)).length() == workflowTransitions.length()
  val workflowCommandTransitionsResolveControlsAndCommands = workflowTransitions.select(transition => workflowCommandTransitionSourceOwnsControl(transition) and workflowCommandTransitionTargetsOwnedCommand(transition)).length() == workflowTransitions.length()
  def workflowSliceHasKind(sliceSlug, kind) = workflowSliceDetails.select(detail => detail.slug == sliceSlug and detail.kind == kind).length() > 0
  def workflowStateViewCommandTransitionTargetsStateChange(transition) = transition.kind != Command or not(workflowSliceHasKind(transition.source, SliceStateView)) or workflowSliceHasKind(transition.target, SliceStateChange)
  val workflowStateViewCommandTransitionsTargetStateChanges = workflowTransitions.select(transition => workflowStateViewCommandTransitionTargetsStateChange(transition)).length() == workflowTransitions.length()
  def workflowEventTransitionIsSharedByEndpoints(transition) = transition.kind != Event or (workflowOwnsDefinition(transition.source, OwnedEvent, transition.trigger) and workflowOwnsDefinition(transition.target, OwnedEvent, transition.trigger))
  val workflowEventTransitionsAreSharedByEndpointSlices = workflowTransitions.select(transition => workflowEventTransitionIsSharedByEndpoints(transition)).length() == workflowTransitions.length()
  def workflowEventTransitionSourceParticipates(transition) = transition.kind != Event or workflowEventDefinitionParticipates(transition.source, transition.trigger)
  def workflowEventTransitionTargetParticipates(transition) = transition.kind != Event or workflowEventDefinitionParticipates(transition.target, transition.trigger)
  val workflowEventTransitionsHaveParticipatingEndpointEvents = workflowTransitions.select(transition => workflowEventTransitionSourceParticipates(transition) and workflowEventTransitionTargetParticipates(transition)).length() == workflowTransitions.length()
  def workflowNavigationTransitionSourceOwnsControl(transition) = transition.kind != Navigation or workflowOwnsDefinition(transition.source, OwnedControl, workflowNavigationSourceControl(transition))
  def workflowNavigationTransitionTargetsOwnedView(transition) = transition.kind != Navigation or workflowOwnsDefinition(transition.target, OwnedView, workflowNavigationTargetView(transition))
  def workflowNavigationTransitionTargetsEntryView(transition) = transition.kind != Navigation or workflowOwnsEntryView(transition.target, workflowNavigationTargetView(transition))
  val workflowNavigationTransitionsResolveControlsAndViews = workflowTransitions.select(transition => workflowNavigationTransitionSourceOwnsControl(transition) and workflowNavigationTransitionTargetsOwnedView(transition)).length() == workflowTransitions.length()
  val workflowNavigationTransitionsResolveToEntryViews = workflowTransitions.select(transition => workflowNavigationTransitionTargetsEntryView(transition)).length() == workflowTransitions.length()
  def workflowExternalTriggerDeclaresPayloadContract(transition) = transition.kind != ExternalTrigger or (transition.payloadContract != "" and workflowOwnsDefinition(transition.source, OwnedExternalPayload, transition.payloadContract))
  val workflowExternalTriggersDeclarePayloadContracts = workflowTransitions.select(transition => workflowExternalTriggerDeclaresPayloadContract(transition)).length() == workflowTransitions.length()
  def workflowExternalTriggerPayloadContractHasProvenance(transition) = transition.kind != ExternalTrigger or workflowOwnedDefinitions.select(definition => definition.sourceSlice == transition.source and definition.definitionKind == OwnedExternalPayload and definition.definitionName == transition.payloadContract and definition.sourceProvenance != "").length() > 0
  val workflowExternalTriggerPayloadContractsHaveProvenance = workflowTransitions.select(transition => workflowExternalTriggerPayloadContractHasProvenance(transition)).length() == workflowTransitions.length()
  def workflowTransitionRequiresEvidence(transition) = transition.kind == Event or transition.kind == Command or transition.kind == Navigation
  def workflowTransitionEvidenceMatches(transition, evidence) = evidence.source == transition.source and evidence.target == transition.target and evidence.kind == transition.kind and evidence.trigger == transition.trigger and (transition.kind != Navigation or ((evidence.sourceControl == "" or evidence.sourceControl == workflowNavigationSourceControl(transition)) and (evidence.targetView == "" or evidence.targetView == workflowNavigationTargetView(transition)))) and evidence.sourceEvidence != "" and evidence.targetEvidence != ""
  def workflowTransitionHasRequiredEvidence(transition) = not(workflowTransitionRequiresEvidence(transition)) or workflowTransitionEvidences.select(evidence => workflowTransitionEvidenceMatches(transition, evidence)).length() > 0
  val workflowTransitionsHaveRequiredEvidence = workflowTransitions.select(transition => workflowTransitionHasRequiredEvidence(transition)).length() == workflowTransitions.length()
  def workflowEntryLifecycleStateCovered(state) = workflowEntryLifecycleStates.select(coverage => coverage.state == state and workflowSliceSlugs.select(step => step == coverage.step).length() > 0 and coverage.evidence != "").length() > 0
  val workflowEntryLifecycleStatesCoverRequiredStates = not(workflowRequiresEntryLifecycleCoverage) or requiredEntryLifecycleStates.select(state => workflowEntryLifecycleStateCovered(state)).length() == requiredEntryLifecycleStates.length()
  var modelState: int
  action init = modelState' = 0
  action step = modelState' = modelState
}
