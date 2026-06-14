namespace LoadStateFromSnapshot

-- EMC-DIGEST: slice:name=Load State from Snapshot;slug=load-state-from-snapshot;kind=state_change;description=Given snapshot metadata returned, When snapshot file is deserialised into in-memory state, Then SnapshotLoaded event emitted with snapshot_id and event_count as the replay start offset
-- EMC generated Lean4 business slice model.
def sliceName := "Load State from Snapshot"

def sliceSlug := "load-state-from-snapshot"

inductive SliceKindName where
  | stateView
  | stateChange
  | translation
  | automation
deriving BEq, DecidableEq, Repr

def sliceKind : SliceKindName := SliceKindName.stateChange

def sliceDescription := "Given snapshot metadata returned, When snapshot file is deserialised into in-memory state, Then SnapshotLoaded event emitted with snapshot_id and event_count as the replay start offset"

structure EventModelScenario where
  name : String
  givenSteps : List String
  whenSteps : List String
  thenSteps : List String
  readStreams : List String
  writtenStreams : List String
  contractKind : String
  coveredDefinition : String
  errorReferences : List String

structure BitLevelDataFlow where
  datum : String
  sourceKind : String
  source : String
  transformationSemantics : String
  target : String
  bitEncoding : String

inductive CommandInputSourceKind where
  | actor
  | session
  | generated
  | externalPayload
  | eventStreamState
  | invocationArgument
deriving BEq, DecidableEq, Repr

structure CommandInput where
  name : String
  sourceKind : CommandInputSourceKind
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

structure CommandErrorDefinition where
  name : String
  scenarioName : String
  recoveryKind : String

structure SliceEventReference where
  name : String

structure SliceStreamReference where
  name : String

structure CommandDefinition where
  name : String
  inputs : List CommandInput
  emittedEvents : List SliceEventReference
  observedStreams : List SliceStreamReference
  errors : List CommandErrorDefinition
  singleton : Bool
  repeatBehavior : String

structure SliceCommandReference where
  name : String

structure OutcomeDefinition where
  label : String
  eventSet : List String
  externallyRelevant : Bool

structure StreamDefinition where
  name : String

structure EventAttribute where
  name : String
  sourceKind : String
  sourceName : String
  sourceField : String
  generatedSourceKind : String
  provenanceDescription : String

structure ExternalPayloadField where
  name : String
  provenanceDescription : String
  bitEncoding : String

structure ExternalPayloadDefinition where
  name : String
  fields : List ExternalPayloadField

structure EventDefinition where
  name : String
  stream : String
  attributes : List EventAttribute
  observed : Bool
  shared : Bool

structure ReadModelField where
  name : String
  sourceKind : String
  sourceEvent : String
  sourceAttribute : String
  derivationRule : String
  derivationSourceFields : List String
  absenceEvent : String
  derivationScenarioName : String
  absenceScenarioName : String
  provenanceDescription : String

structure ReadModelDefinition where
  name : String
  fields : List ReadModelField
  transitive : Bool
  relationshipFields : List String
  transitiveRule : String
  exampleScenarioName : String

structure SliceReadModelReference where
  name : String

structure ViewField where
  name : String
  sourceKind : String
  sourceReadModel : String
  sourceField : String
  sketchToken : String
  provenanceDescription : String
  bitEncoding : String

structure ControlInputProvision where
  name : String
  sourceKind : CommandInputSourceKind
  sourceDescription : String
  sketchToken : String
  visibleToActor : Bool
  decisionField : Bool

structure NavigationTarget where
  targetType : String
  targetName : String
  externalWorkflowName : String
  externalSystemName : String
  handoffContract : String

structure ControlDefinition where
  name : String
  commandName : String
  inputs : List ControlInputProvision
  handledErrors : List String
  recoveryBehavior : String
  sketchToken : String
  navigation : NavigationTarget

structure ViewDefinition where
  name : String
  readModels : List String
  fields : List ViewField
  controls : List ControlDefinition
  sketchTokens : List String
  localStates : List String
  filters : List String

structure SliceViewReference where
  name : String

structure AutomationDefinition where
  name : String
  triggerName : String
  commandName : String
  handledErrors : List String
  reactionDescription : String

structure TranslationDefinition where
  name : String
  externalEventName : String
  payloadContractName : String
  commandName : String

structure BoardElement where
  name : String
  kind : String
  lane : String
  declaredName : String
  mainPath : Bool

structure BoardConnection where
  source : String
  sourceKind : String
  target : String
  targetKind : String

def sliceCommands : List SliceCommandReference := []

def sliceCommandNames : List String := sliceCommands.map (fun commandRef => commandRef.name)

def sliceCommandDefinitions : List CommandDefinition := []

def sliceAutomations : List AutomationDefinition := []

def sliceTranslations : List TranslationDefinition := []

def canonicalBoardLanes : List String := ["ux","actions","events"]

def sliceBoardElements : List BoardElement := []

def sliceBoardConnections : List BoardConnection := []

def sliceReferencedCommands : List SliceCommandReference := []

def sliceReferencedCommandNames : List String := sliceReferencedCommands.map (fun commandRef => commandRef.name)

def sliceOutcomeDefinitions : List OutcomeDefinition := []

def allowedCommandInputSourceKinds : List CommandInputSourceKind := [CommandInputSourceKind.actor,CommandInputSourceKind.session,CommandInputSourceKind.generated,CommandInputSourceKind.externalPayload,CommandInputSourceKind.eventStreamState,CommandInputSourceKind.invocationArgument]

def allowedRecoveryKinds : List String := ["retry","stay_on_screen","navigation","explicit_recovery_action"]

def allowedSingletonRepeatBehaviors : List String := ["already_exists_error","idempotent"]

def sliceEvents : List SliceEventReference := []

def sliceEventNames : List String := sliceEvents.map (fun eventRef => eventRef.name)

def sliceStreams : List StreamDefinition := []

def sliceExternalPayloads : List ExternalPayloadDefinition := []

def sliceEventDefinitions : List EventDefinition := []

def storedEventFactSourceKinds : List String := ["command_input","external_payload","generated","session","derivation"]

def allowedEventAttributeSourceKinds : List String := storedEventFactSourceKinds

def sliceReadModels : List SliceReadModelReference := []

def sliceReadModelNames : List String := sliceReadModels.map (fun readModelRef => readModelRef.name)

def sliceReadModelDefinitions : List ReadModelDefinition := []

def allowedReadModelFieldSourceKinds : List String := ["event_attribute","derivation","absence_default"]

def sliceViews : List SliceViewReference := []

def sliceViewNames : List String := sliceViews.map (fun viewRef => viewRef.name)

def sliceViewDefinitions : List ViewDefinition := []

def allowedViewFieldSourceKinds : List String := ["read_model"]

def allowedControlInputSourceKinds : List CommandInputSourceKind := [CommandInputSourceKind.actor,CommandInputSourceKind.session,CommandInputSourceKind.generated,CommandInputSourceKind.externalPayload,CommandInputSourceKind.eventStreamState,CommandInputSourceKind.invocationArgument]

def allowedNavigationTargetTypes : List String := ["modeled_view","local_view_state","external_system","external_workflow"]

def sliceAcceptanceScenarios : List EventModelScenario := []

def sliceContractScenarios : List EventModelScenario := []

def sliceBitLevelDataFlows : List BitLevelDataFlow := []

def scenarioHasGwt (scenario : EventModelScenario) : Bool := scenario.name.isEmpty == false && scenario.givenSteps.isEmpty == false && scenario.whenSteps.isEmpty == false && scenario.thenSteps.isEmpty == false

def sliceScenariosHaveGwt : Bool := sliceAcceptanceScenarios.all scenarioHasGwt && sliceContractScenarios.all scenarioHasGwt

def scenarioNameCount (name : String) (scenarios : List EventModelScenario) : Nat := (scenarios.filter (fun scenario => scenario.name == name)).length

def scenarioNamesAreUnique (scenarios : List EventModelScenario) : Bool := scenarios.all (fun scenario => scenarioNameCount scenario.name scenarios == 1)

def sliceScenarioNamesAreUnique : Bool := scenarioNamesAreUnique (sliceAcceptanceScenarios ++ sliceContractScenarios)

def stringNameCount (name : String) (names : List String) : Nat := (names.filter (fun other => other == name)).length

def definitionNamesAreUnique (names : List String) : Bool := names.all (fun name => stringNameCount name names == 1)

def sliceOwnedCommandNames : List String := sliceCommandDefinitions.map (fun command => command.name)

def sliceOwnedEventNames : List String := sliceEventDefinitions.map (fun event => event.name)

def sliceOwnedStreamNames : List String := sliceStreams.map (fun stream => stream.name)

def sliceOwnedExternalPayloadNames : List String := sliceExternalPayloads.map (fun payload => payload.name)

def sliceOwnedReadModelNames : List String := sliceReadModelDefinitions.map (fun readModel => readModel.name)

def sliceOwnedViewNames : List String := sliceViewDefinitions.map (fun view => view.name)

def sliceOwnedAutomationNames : List String := sliceAutomations.map (fun automation => automation.name)

def sliceOwnedTranslationNames : List String := sliceTranslations.map (fun translation => translation.name)

def sliceOwnedControlNames : List String := sliceViewDefinitions.flatMap (fun view => view.controls.map (fun control => control.name))

def sliceNamedDefinitionsAreUniquelyOwned : Bool := definitionNamesAreUnique sliceCommandNames && definitionNamesAreUnique sliceOwnedCommandNames && definitionNamesAreUnique sliceEventNames && definitionNamesAreUnique sliceOwnedEventNames && definitionNamesAreUnique sliceOwnedStreamNames && definitionNamesAreUnique sliceOwnedExternalPayloadNames && definitionNamesAreUnique sliceReadModelNames && definitionNamesAreUnique sliceOwnedReadModelNames && definitionNamesAreUnique sliceViewNames && definitionNamesAreUnique sliceOwnedViewNames && definitionNamesAreUnique sliceOwnedAutomationNames && definitionNamesAreUnique sliceOwnedTranslationNames && definitionNamesAreUnique sliceOwnedControlNames

def scenarioStreamResolves (streamName : String) : Bool := sliceStreams.any (fun stream => stream.name == streamName)

def scenarioStreamsResolve (scenario : EventModelScenario) : Bool := scenario.readStreams.all scenarioStreamResolves && scenario.writtenStreams.all scenarioStreamResolves

def stateChangeScenarioNamesStreams (scenario : EventModelScenario) : Bool := sliceKind != SliceKindName.stateChange || (scenario.readStreams.isEmpty == false && scenario.writtenStreams.isEmpty == false)

def sliceScenarioStreamsResolve : Bool := (sliceAcceptanceScenarios ++ sliceContractScenarios).all scenarioStreamsResolve

def stateChangeScenariosNameStreams : Bool := (sliceAcceptanceScenarios ++ sliceContractScenarios).all stateChangeScenarioNamesStreams

def acceptanceScenariosAreUserFacing : Bool := sliceAcceptanceScenarios.all (fun scenario => scenario.contractKind.isEmpty && scenario.coveredDefinition.isEmpty)

def scenarioCoversContract (contractKind : String) (definitionName : String) (scenario : EventModelScenario) : Bool := scenario.contractKind == contractKind && scenario.coveredDefinition == definitionName

def readModelHasProjectorContract (readModel : ReadModelDefinition) : Bool := sliceContractScenarios.any (scenarioCoversContract "projector" readModel.name)

def stateViewReadModelsHaveProjectorContracts : Bool := sliceKind != SliceKindName.stateView || sliceReadModelDefinitions.all readModelHasProjectorContract

def contractScenarioTargetsKnownDefinition (scenario : EventModelScenario) : Bool := (scenario.contractKind == "projector" && (sliceReadModelNames.contains scenario.coveredDefinition || sliceReadModelDefinitions.any (fun readModel => readModel.name == scenario.coveredDefinition))) || (scenario.contractKind == "command" && (sliceCommandNames.contains scenario.coveredDefinition || sliceCommandDefinitions.any (fun command => command.name == scenario.coveredDefinition))) || (scenario.contractKind == "automation" && sliceAutomations.any (fun automation => automation.name == scenario.coveredDefinition)) || (scenario.contractKind == "translation" && sliceTranslations.any (fun translation => translation.name == scenario.coveredDefinition)) || (scenario.contractKind == "derivation" && scenario.coveredDefinition.isEmpty == false && sliceReadModelDefinitions.any (fun readModel => readModel.fields.any (fun field => field.sourceKind == "derivation" && field.derivationScenarioName == scenario.name))) || (scenario.contractKind == "absence" && scenario.coveredDefinition.isEmpty == false && sliceReadModelDefinitions.any (fun readModel => readModel.fields.any (fun field => field.sourceKind == "absence_default" && field.absenceScenarioName == scenario.name))) || (scenario.contractKind == "transitive" && sliceReadModelDefinitions.any (fun readModel => readModel.transitive && readModel.name == scenario.coveredDefinition && readModel.exampleScenarioName == scenario.name))

def contractScenariosTargetKnownDefinitions : Bool := sliceContractScenarios.all contractScenarioTargetsKnownDefinition

def commandHasContractScenario (command : CommandDefinition) : Bool := sliceContractScenarios.any (scenarioCoversContract "command" command.name)

def automationHasContractScenario (automation : AutomationDefinition) : Bool := sliceContractScenarios.any (scenarioCoversContract "automation" automation.name)

def translationHasContractScenario (translation : TranslationDefinition) : Bool := sliceContractScenarios.any (scenarioCoversContract "translation" translation.name)

def derivationFieldHasContractScenario (field : ReadModelField) : Bool := field.sourceKind != "derivation" || sliceContractScenarios.any (fun scenario => scenario.contractKind == "derivation" && scenario.coveredDefinition.isEmpty == false && scenario.name == field.derivationScenarioName)

def contractScenariosCoverModeledContracts : Bool := sliceCommandDefinitions.all commandHasContractScenario && sliceAutomations.all automationHasContractScenario && sliceTranslations.all translationHasContractScenario && sliceReadModelDefinitions.all (fun readModel => readModel.fields.all derivationFieldHasContractScenario)

def commandInputHasAllowedSource (input : CommandInput) : Bool := allowedCommandInputSourceKinds.contains input.sourceKind

def commandInputHasProvenance (input : CommandInput) : Bool := input.name.isEmpty == false && input.sourceDescription.isEmpty == false && input.provenanceChain.isEmpty == false

def commandInputSessionInputHasDescription (input : CommandInput) : Bool := input.sourceKind != CommandInputSourceKind.session || input.sourceDescription.isEmpty == false

def commandHasIssuingControl (command : CommandDefinition) : Bool := sliceViewDefinitions.any (fun view => view.controls.any (fun control => control.commandName == command.name))

def commandInputWithoutIssuingControlHasProvenance (command : CommandDefinition) (input : CommandInput) : Bool := commandHasIssuingControl command || commandInputHasProvenance input

def commandInputsHaveAllowedSources : Bool := sliceCommandDefinitions.all (fun command => command.inputs.all commandInputHasAllowedSource)

def commandInputsHaveProvenance : Bool := sliceCommandDefinitions.all (fun command => command.inputs.all commandInputHasProvenance)

def commandInputsWithoutIssuingControlsHaveProvenance : Bool := sliceCommandDefinitions.all (fun command => command.inputs.all (commandInputWithoutIssuingControlHasProvenance command))

def commandSessionInputsHaveDescriptions : Bool := sliceCommandDefinitions.all (fun command => command.inputs.all commandInputSessionInputHasDescription)

def commandInputTracesToInvocationSource (input : CommandInput) : Bool := allowedCommandInputSourceKinds.contains input.sourceKind && input.provenanceChain.isEmpty == false

def commandInputsTraceToInvocationSources : Bool := sliceCommandDefinitions.all (fun command => command.inputs.all commandInputTracesToInvocationSource)

def commandObservedStreamNames (command : CommandDefinition) : List String := command.observedStreams.map (fun streamRef => streamRef.name)

def commandInputEventStreamSourceResolves (command : CommandDefinition) (input : CommandInput) : Bool := input.sourceKind != CommandInputSourceKind.eventStreamState || ((commandObservedStreamNames command).isEmpty == false && (commandObservedStreamNames command).all scenarioStreamResolves && input.eventStreamSourceEvent.isEmpty == false && input.eventStreamSourceAttribute.isEmpty == false && sliceEventDefinitions.any (fun event => event.name == input.eventStreamSourceEvent && event.attributes.any (fun eventAttribute => eventAttribute.name == input.eventStreamSourceAttribute)))

def commandInputExternalPayloadSourceResolves (input : CommandInput) : Bool := input.sourceKind != CommandInputSourceKind.externalPayload || (input.externalPayloadSourceName.isEmpty == false && input.externalPayloadSourceField.isEmpty == false && sliceExternalPayloads.any (fun payload => payload.name == input.externalPayloadSourceName && payload.fields.any (fun payloadField => payloadField.name == input.externalPayloadSourceField)))

def commandInputGeneratedSourceHasCoordinates (input : CommandInput) : Bool := input.sourceKind != CommandInputSourceKind.generated || (input.generatedSourceName.isEmpty == false && input.generatedSourceField.isEmpty == false)

def commandInputSessionSourceHasCoordinates (input : CommandInput) : Bool := input.sourceKind != CommandInputSourceKind.session || (input.sessionSourceName.isEmpty == false && input.sessionSourceField.isEmpty == false)

def commandInputInvocationArgumentSourceHasCoordinates (input : CommandInput) : Bool := input.sourceKind != CommandInputSourceKind.invocationArgument || (input.invocationArgumentSourceName.isEmpty == false && input.invocationArgumentSourceField.isEmpty == false)

def commandInputsSourcedFromEventStreamsResolve : Bool := sliceCommandDefinitions.all (fun command => command.inputs.all (commandInputEventStreamSourceResolves command))

def commandInputsSourcedFromExternalPayloadsResolve : Bool := sliceCommandDefinitions.all (fun command => command.inputs.all commandInputExternalPayloadSourceResolves)

def commandInputsSourcedFromGeneratedValuesHaveCoordinates : Bool := sliceCommandDefinitions.all (fun command => command.inputs.all commandInputGeneratedSourceHasCoordinates)

def commandInputsSourcedFromSessionValuesHaveCoordinates : Bool := sliceCommandDefinitions.all (fun command => command.inputs.all commandInputSessionSourceHasCoordinates)

def commandInputsSourcedFromInvocationArgumentsHaveCoordinates : Bool := sliceCommandDefinitions.all (fun command => command.inputs.all commandInputInvocationArgumentSourceHasCoordinates)

def bitLevelFlowCoversTarget (target : String) (datum : String) : Bool := sliceBitLevelDataFlows.any (fun flow => flow.target == target && flow.datum == datum && flow.sourceKind.isEmpty == false && flow.source.isEmpty == false && flow.transformationSemantics.isEmpty == false && flow.bitEncoding.isEmpty == false)

def commandInputHasBitLevelFlow (command : CommandDefinition) (input : CommandInput) : Bool := bitLevelFlowCoversTarget command.name input.name

def commandErrorHasDeclaration (error : CommandErrorDefinition) : Bool := error.name.isEmpty == false && error.scenarioName.isEmpty == false && error.recoveryKind.isEmpty == false

def commandErrorHasAllowedRecovery (error : CommandErrorDefinition) : Bool := allowedRecoveryKinds.contains error.recoveryKind

def commandErrorsAreDeclared : Bool := sliceCommandDefinitions.all (fun command => command.errors.all commandErrorHasDeclaration)

def commandErrorsHaveAllowedRecovery : Bool := sliceCommandDefinitions.all (fun command => command.errors.all commandErrorHasAllowedRecovery)

def scenarioNameIsModeled (scenarioName : String) : Bool := (sliceAcceptanceScenarios ++ sliceContractScenarios).any (fun scenario => scenario.name == scenarioName)

def commandErrorHasScenarioCoverage (command : CommandDefinition) (error : CommandErrorDefinition) : Bool := sliceContractScenarios.any (fun scenario => scenario.name == error.scenarioName && scenario.contractKind == "command" && scenario.coveredDefinition == command.name && scenario.errorReferences.contains error.name)

def commandErrorsHaveScenarioCoverage : Bool := sliceCommandDefinitions.all (fun command => command.errors.all (commandErrorHasScenarioCoverage command))

def scenarioErrorReferenceIsDeclared (scenario : EventModelScenario) (errorName : String) : Bool := scenario.contractKind != "command" || sliceCommandDefinitions.any (fun command => command.name == scenario.coveredDefinition && command.errors.any (fun error => error.name == errorName))

def scenarioErrorReferencesAreDeclaredForScenario (scenario : EventModelScenario) : Bool := scenario.errorReferences.all (scenarioErrorReferenceIsDeclared scenario)

def scenarioErrorReferencesAreDeclared : Bool := sliceContractScenarios.all scenarioErrorReferencesAreDeclaredForScenario

def singletonCommandDeclaresRepeatBehavior (command : CommandDefinition) : Bool := command.singleton == false || allowedSingletonRepeatBehaviors.contains command.repeatBehavior

def singletonCommandsDeclareRepeatBehavior : Bool := sliceCommandDefinitions.all singletonCommandDeclaresRepeatBehavior

def commandEmittedEventNames (command : CommandDefinition) : List String := command.emittedEvents.map (fun eventRef => eventRef.name)

def automationHasTrigger (automation : AutomationDefinition) : Bool := automation.name.isEmpty == false && automation.triggerName.isEmpty == false && automation.reactionDescription.isEmpty == false

def automationIssuesKnownCommand (automation : AutomationDefinition) : Bool := sliceCommandNames.contains automation.commandName || sliceReferencedCommandNames.contains automation.commandName || sliceCommandDefinitions.any (fun command => command.name == automation.commandName)

def automationHandlesCommandErrors (automation : AutomationDefinition) (command : CommandDefinition) : Bool := command.name != automation.commandName || command.errors.all (fun error => automation.handledErrors.contains error.name)

def automationSlicesDeclareTriggers : Bool := sliceKind != SliceKindName.automation || (sliceAutomations.isEmpty == false && sliceAutomations.all automationHasTrigger)

def automationSlicesRepresentOneReaction : Bool := sliceKind != SliceKindName.automation || sliceAutomations.length == 1

def automationsIssueKnownCommands : Bool := sliceAutomations.all automationIssuesKnownCommand

def automationsHandleCommandErrors : Bool := sliceAutomations.all (fun automation => sliceCommandDefinitions.all (automationHandlesCommandErrors automation))

def externalPayloadFieldHasProvenance (field : ExternalPayloadField) : Bool := field.name.isEmpty == false && field.provenanceDescription.isEmpty == false && field.bitEncoding.isEmpty == false

def translationHasExternalContract (translation : TranslationDefinition) : Bool := translation.name.isEmpty == false && translation.externalEventName.isEmpty == false && translation.payloadContractName.isEmpty == false && sliceExternalPayloads.any (fun payload => payload.name == translation.payloadContractName)

def externalBoundaryHasPayloadContractAndFieldProvenance (translation : TranslationDefinition) : Bool := translationHasExternalContract translation && sliceExternalPayloads.any (fun payload => payload.name == translation.payloadContractName && payload.fields.isEmpty == false && payload.fields.all externalPayloadFieldHasProvenance)

def externalBoundariesHavePayloadContractsAndFieldProvenance : Bool := sliceTranslations.all externalBoundaryHasPayloadContractAndFieldProvenance

def translationTargetsKnownCommand (translation : TranslationDefinition) : Bool := sliceCommandNames.contains translation.commandName || sliceReferencedCommandNames.contains translation.commandName || sliceCommandDefinitions.any (fun command => command.name == translation.commandName)

def translationReferencesObservedExternalEvent (translation : TranslationDefinition) : Bool := sliceEventDefinitions.any (fun event => event.name == translation.externalEventName && event.observed)

def translationSlicesDeclareExternalContracts : Bool := sliceKind != SliceKindName.translation || (sliceTranslations.isEmpty == false && sliceTranslations.all translationHasExternalContract)

def translationsTargetKnownCommands : Bool := sliceTranslations.all translationTargetsKnownCommand

def translationsReferenceObservedExternalEvents : Bool := sliceTranslations.all translationReferencesObservedExternalEvent

def boardElementLaneMatchesKind (element : BoardElement) : Bool := (element.kind == "view" && element.lane == "ux") || (element.kind == "automation" && element.lane == "ux") || (element.kind == "external_event" && element.lane == "ux") || (element.kind == "command" && element.lane == "actions") || (element.kind == "read_model" && element.lane == "actions") || (element.kind == "event" && element.lane == "events")

def boardElementReferencesDeclaration (element : BoardElement) : Bool := (element.kind == "view" && (sliceViewNames.contains element.declaredName || sliceViewDefinitions.any (fun view => view.name == element.declaredName))) || (element.kind == "automation" && sliceAutomations.any (fun automation => automation.name == element.declaredName)) || (element.kind == "external_event" && sliceEventDefinitions.any (fun event => event.name == element.declaredName && event.observed)) || (element.kind == "command" && (sliceCommandNames.contains element.declaredName || sliceReferencedCommandNames.contains element.declaredName || sliceCommandDefinitions.any (fun command => command.name == element.declaredName))) || (element.kind == "read_model" && (sliceReadModelNames.contains element.declaredName || sliceReadModelDefinitions.any (fun readModel => readModel.name == element.declaredName))) || (element.kind == "event" && (sliceEventNames.contains element.declaredName || sliceEventDefinitions.any (fun event => event.name == element.declaredName && (event.observed || event.shared))))

def automationBoardElementIsDeclaredAutomation (element : BoardElement) : Bool := element.kind != "automation" || sliceAutomations.any (fun automation => automation.name == element.declaredName)

def automationBoardElementsAreDeclaredAutomations : Bool := sliceBoardElements.all automationBoardElementIsDeclaredAutomation

def externalBoardElementIsObservedEvent (element : BoardElement) : Bool := element.kind != "external_event" || sliceEventDefinitions.any (fun event => event.name == element.declaredName && event.observed)

def externalBoardElementsAreObservedEvents : Bool := sliceBoardElements.all externalBoardElementIsObservedEvent

def boardConnectionHasAllowedShape (connection : BoardConnection) : Bool := (connection.sourceKind == "view" && connection.targetKind == "command") || (connection.sourceKind == "automation" && connection.targetKind == "command") || (connection.sourceKind == "external_event" && connection.targetKind == "command") || (connection.sourceKind == "workflow_trigger" && connection.targetKind == "command") || (connection.sourceKind == "command" && connection.targetKind == "event") || (connection.sourceKind == "event" && connection.targetKind == "read_model") || (connection.sourceKind == "read_model" && connection.targetKind == "view")

def commandEventBoardEdgeMatchesEmission (connection : BoardConnection) : Bool := connection.sourceKind != "command" || connection.targetKind != "event" || sliceCommandDefinitions.any (fun command => command.name == connection.source && (commandEmittedEventNames command).contains connection.target)

def commandEventBoardEdgesMatchEmissions : Bool := sliceBoardConnections.all commandEventBoardEdgeMatchesEmission

def eventReadModelBoardEdgeMatchesProjection (connection : BoardConnection) : Bool := connection.sourceKind != "event" || connection.targetKind != "read_model" || sliceReadModelDefinitions.any (fun readModel => readModel.name == connection.target && readModel.fields.any (fun field => field.sourceEvent == connection.source))

def eventReadModelBoardEdgesMatchProjectionSources : Bool := sliceBoardConnections.all eventReadModelBoardEdgeMatchesProjection

def externalEventCommandBoardEdgeMatchesTranslation (connection : BoardConnection) : Bool := connection.sourceKind != "external_event" || connection.targetKind != "command" || sliceTranslations.any (fun translation => translation.externalEventName == connection.source && translation.commandName == connection.target)

def externalEventTriggersMatchTranslations : Bool := sliceBoardConnections.all externalEventCommandBoardEdgeMatchesTranslation

def externalEventDoesNotUpdateReadModel (connection : BoardConnection) : Bool := connection.sourceKind != "event" || connection.targetKind != "read_model" || sliceEventDefinitions.any (fun event => event.name == connection.source && event.observed) == false

def externalEventsDoNotUpdateReadModels : Bool := sliceBoardConnections.all externalEventDoesNotUpdateReadModel

def viewCommandBoardEdgeMatchesControl (connection : BoardConnection) : Bool := connection.sourceKind != "view" || connection.targetKind != "command" || sliceViewDefinitions.any (fun view => view.name == connection.source && view.controls.any (fun control => control.commandName == connection.target))

def viewCommandBoardEdgesMatchControls : Bool := sliceBoardConnections.all viewCommandBoardEdgeMatchesControl

def boardLanesAreCanonical : Bool := canonicalBoardLanes == ["ux","actions","events"]

def boardElementsUseCanonicalLanes : Bool := sliceBoardElements.all (fun element => canonicalBoardLanes.contains element.lane && boardElementLaneMatchesKind element)

def boardElementsReferenceDeclarations : Bool := sliceBoardElements.all boardElementReferencesDeclaration

def boardConnectionsHaveCausalSemantics : Bool := sliceBoardConnections.all (fun connection => boardConnectionHasAllowedShape connection && commandEventBoardEdgeMatchesEmission connection && eventReadModelBoardEdgeMatchesProjection connection && externalEventCommandBoardEdgeMatchesTranslation connection && externalEventDoesNotUpdateReadModel connection && viewCommandBoardEdgeMatchesControl connection)

def readModelViewConnectionHasIncomingEventUpdate (connection : BoardConnection) : Bool := connection.sourceKind != "read_model" || connection.targetKind != "view" || sliceBoardConnections.any (fun incoming => incoming.target == connection.source && incoming.targetKind == "read_model" && incoming.sourceKind == "event")

def readModelsFeedingViewsHaveIncomingEventUpdates : Bool := sliceBoardConnections.all readModelViewConnectionHasIncomingEventUpdate

def commandsHaveIncomingTriggers : Bool := sliceBoardElements.all (fun element => element.kind != "command" || sliceBoardConnections.any (fun connection => connection.target == element.name && connection.targetKind == "command" && (connection.sourceKind == "view" || connection.sourceKind == "automation" || connection.sourceKind == "external_event" || connection.sourceKind == "workflow_trigger")))

def mainPathBoardHasNoDisconnectedIslands : Bool := sliceBoardElements.all (fun element => element.mainPath == false || sliceBoardConnections.any (fun connection => connection.source == element.name || connection.target == element.name))

def outcomeLabelCount (label : String) : Nat := (sliceOutcomeDefinitions.filter (fun outcome => outcome.label == label)).length

def outcomeLabelsAreUnique : Bool := sliceOutcomeDefinitions.all (fun outcome => outcomeLabelCount outcome.label == 1)

def outcomeEventSetsAreNonEmpty : Bool := sliceOutcomeDefinitions.all (fun outcome => outcome.eventSet.isEmpty == false)

def sameOutcomeEventSet (left : OutcomeDefinition) (right : OutcomeDefinition) : Bool := left.eventSet.all (fun eventName => right.eventSet.contains eventName) && right.eventSet.all (fun eventName => left.eventSet.contains eventName)

def eventIsKnownToSlice (eventName : String) : Bool := sliceEventNames.contains eventName || sliceEventDefinitions.any (fun event => event.name == eventName && (event.observed || event.shared))

def outcomeEventSetsAreDistinct : Bool := sliceOutcomeDefinitions.all (fun outcome => sliceOutcomeDefinitions.all (fun other => outcome.label == other.label || sameOutcomeEventSet outcome other == false))

def outcomeEventsAreKnownToSlice : Bool := sliceOutcomeDefinitions.all (fun outcome => outcome.eventSet.all eventIsKnownToSlice)

def eventReferencesKnownStream (event : EventDefinition) : Bool := sliceStreams.any (fun stream => stream.name == event.stream)

def eventAttributeHasAllowedSource (eventAttribute : EventAttribute) : Bool := allowedEventAttributeSourceKinds.contains eventAttribute.sourceKind

def eventAttributeHasProvenance (eventAttribute : EventAttribute) : Bool := eventAttribute.name.isEmpty == false && eventAttribute.sourceKind.isEmpty == false && eventAttribute.sourceName.isEmpty == false && eventAttribute.provenanceDescription.isEmpty == false

def commandEmittedEventIsKnown (eventName : String) : Bool := sliceEventNames.contains eventName || sliceEventDefinitions.any (fun event => event.name == eventName)

def eventProducedByCommand (event : EventDefinition) : Bool := event.observed || event.shared || sliceCommandDefinitions.any (fun command => (commandEmittedEventNames command).contains event.name)

def commandInputReferencesAttributeSource (event : EventDefinition) (eventAttribute : EventAttribute) (command : CommandDefinition) : Bool := (commandEmittedEventNames command).contains event.name && command.inputs.any (fun input => input.name == eventAttribute.sourceName)

def externalPayloadFieldsHaveProvenance : Bool := sliceExternalPayloads.all (fun payload => payload.name.isEmpty == false && payload.fields.all externalPayloadFieldHasProvenance)

def externalPayloadFieldHasBitLevelFlow (payload : ExternalPayloadDefinition) (field : ExternalPayloadField) : Bool := bitLevelFlowCoversTarget payload.name field.name

def externalPayloadFieldIsDeclared (eventAttribute : EventAttribute) : Bool := sliceExternalPayloads.any (fun payload => payload.name == eventAttribute.sourceName && payload.fields.any (fun field => field.name == eventAttribute.sourceField))

def eventAttributeSourceIsComplete (event : EventDefinition) (eventAttribute : EventAttribute) : Bool := (eventAttribute.sourceKind == "command_input" && eventAttribute.sourceName.isEmpty == false && eventAttribute.sourceField.isEmpty == false && sliceCommandDefinitions.any (commandInputReferencesAttributeSource event eventAttribute)) || (eventAttribute.sourceKind == "external_payload" && eventAttribute.sourceName.isEmpty == false && eventAttribute.sourceField.isEmpty == false && externalPayloadFieldIsDeclared eventAttribute) || (eventAttribute.sourceKind == "generated" && eventAttribute.sourceName.isEmpty == false && eventAttribute.generatedSourceKind.isEmpty == false) || (eventAttribute.sourceKind == "session" && eventAttribute.sourceName.isEmpty == false) || (eventAttribute.sourceKind == "derivation" && eventAttribute.sourceName.isEmpty == false && eventAttribute.sourceField.isEmpty == false)

def eventAttributeTracesToStoredFactSource (eventAttribute : EventAttribute) : Bool := storedEventFactSourceKinds.contains eventAttribute.sourceKind

def eventsReferenceKnownStreams : Bool := sliceEventDefinitions.all eventReferencesKnownStream

def commandEmittedEventsAreKnown : Bool := sliceCommandDefinitions.all (fun command => (commandEmittedEventNames command).all commandEmittedEventIsKnown)

def locallyEmittedEventsAreProducedByCommands : Bool := sliceEventDefinitions.all eventProducedByCommand

def eventAttributesHaveAllowedSources : Bool := sliceEventDefinitions.all (fun event => event.attributes.all eventAttributeHasAllowedSource)

def eventAttributesHaveProvenance : Bool := sliceEventDefinitions.all (fun event => event.attributes.all eventAttributeHasProvenance)

def eventAttributeSourcesAreComplete : Bool := sliceEventDefinitions.all (fun event => event.attributes.all (eventAttributeSourceIsComplete event))

def storedEventFactsTraceToOriginalSources : Bool := sliceEventDefinitions.all (fun event => event.attributes.all eventAttributeTracesToStoredFactSource)

def eventAttributeHasBitLevelFlow (event : EventDefinition) (eventAttribute : EventAttribute) : Bool := bitLevelFlowCoversTarget event.name eventAttribute.name

def readModelFieldHasAllowedSource (field : ReadModelField) : Bool := allowedReadModelFieldSourceKinds.contains field.sourceKind

def readModelFieldHasProvenance (field : ReadModelField) : Bool := field.name.isEmpty == false && field.sourceKind.isEmpty == false && field.provenanceDescription.isEmpty == false

def readModelFieldSourceIsComplete (field : ReadModelField) : Bool := (field.sourceKind == "event_attribute" && field.sourceEvent.isEmpty == false && field.sourceAttribute.isEmpty == false) || (field.sourceKind == "derivation" && field.derivationRule.isEmpty == false && field.derivationSourceFields.isEmpty == false) || (field.sourceKind == "absence_default" && field.absenceEvent.isEmpty == false)

def eventAttributeIsDeclared (eventName : String) (attributeName : String) : Bool := sliceEventDefinitions.any (fun event => event.name == eventName && event.attributes.any (fun eventAttribute => eventAttribute.name == attributeName))

def readModelFieldEventAttributeSourceResolves (field : ReadModelField) : Bool := field.sourceKind != "event_attribute" || eventAttributeIsDeclared field.sourceEvent field.sourceAttribute

def readModelFieldDerivationScenarioIsCovered (field : ReadModelField) : Bool := field.sourceKind != "derivation" || (field.derivationScenarioName.isEmpty == false && scenarioNameIsModeled field.derivationScenarioName)

def readModelFieldAbsenceScenarioIsCovered (field : ReadModelField) : Bool := field.sourceKind != "absence_default" || (field.absenceScenarioName.isEmpty == false && scenarioNameIsModeled field.absenceScenarioName)

def readModelFieldsHaveAllowedSources : Bool := sliceReadModelDefinitions.all (fun readModel => readModel.fields.all readModelFieldHasAllowedSource)

def readModelFieldsHaveProvenance : Bool := sliceReadModelDefinitions.all (fun readModel => readModel.fields.all readModelFieldHasProvenance)

def readModelFieldSourcesAreComplete : Bool := sliceReadModelDefinitions.all (fun readModel => readModel.fields.all readModelFieldSourceIsComplete)

def readModelFieldEventAttributeSourcesResolve : Bool := sliceReadModelDefinitions.all (fun readModel => readModel.fields.all readModelFieldEventAttributeSourceResolves)

def derivedReadModelFieldsHaveScenarioCoverage : Bool := sliceReadModelDefinitions.all (fun readModel => readModel.fields.all readModelFieldDerivationScenarioIsCovered)

def absenceReadModelFieldsHaveScenarioCoverage : Bool := sliceReadModelDefinitions.all (fun readModel => readModel.fields.all readModelFieldAbsenceScenarioIsCovered)

def transitiveReadModelHasSemantics (readModel : ReadModelDefinition) : Bool := readModel.transitive == false || (readModel.relationshipFields.isEmpty == false && readModel.transitiveRule.isEmpty == false && readModel.exampleScenarioName.isEmpty == false && scenarioNameIsModeled readModel.exampleScenarioName)

def transitiveReadModelsHaveSemantics : Bool := sliceReadModelDefinitions.all transitiveReadModelHasSemantics

def readModelFieldHasBitLevelFlow (readModel : ReadModelDefinition) (field : ReadModelField) : Bool := bitLevelFlowCoversTarget readModel.name field.name

def viewFieldHasAllowedSource (field : ViewField) : Bool := allowedViewFieldSourceKinds.contains field.sourceKind

def viewFieldHasProvenance (field : ViewField) : Bool := field.name.isEmpty == false && field.sourceKind.isEmpty == false && field.provenanceDescription.isEmpty == false && field.bitEncoding.isEmpty == false

def viewFieldSourceIsComplete (field : ViewField) : Bool := field.sourceKind == "read_model" && field.sourceReadModel.isEmpty == false && field.sourceField.isEmpty == false && field.sketchToken.isEmpty == false

def viewFieldSourceReadModelIsUsed (view : ViewDefinition) (field : ViewField) : Bool := view.readModels.contains field.sourceReadModel && sliceReadModelNames.contains field.sourceReadModel

def viewFieldsHaveAllowedSources : Bool := sliceViewDefinitions.all (fun view => view.fields.all viewFieldHasAllowedSource)

def viewFieldsHaveProvenance : Bool := sliceViewDefinitions.all (fun view => view.fields.all viewFieldHasProvenance)

def viewFieldSourcesAreComplete : Bool := sliceViewDefinitions.all (fun view => view.fields.all viewFieldSourceIsComplete)

def viewFieldsSourceFromUsedReadModels : Bool := sliceViewDefinitions.all (fun view => view.fields.all (viewFieldSourceReadModelIsUsed view))

def viewFieldAppearsInSketch (view : ViewDefinition) (field : ViewField) : Bool := field.sketchToken.isEmpty == false && view.sketchTokens.contains field.sketchToken

def viewHasInformationSketch (view : ViewDefinition) : Bool := view.sketchTokens.isEmpty == false

def viewsHaveInformationSketches : Bool := sliceViewDefinitions.all viewHasInformationSketch

def viewFieldsAppearInSketch : Bool := sliceViewDefinitions.all (fun view => view.fields.all (viewFieldAppearsInSketch view))

def sketchTokenMapsToModeledElement (view : ViewDefinition) (token : String) : Bool := view.fields.any (fun field => field.sketchToken == token) || view.controls.any (fun control => control.sketchToken == token || control.inputs.any (fun input => input.sourceKind == CommandInputSourceKind.actor && input.sketchToken == token))

def viewSketchTokensMapToModeledElements : Bool := sliceViewDefinitions.all (fun view => view.sketchTokens.all (sketchTokenMapsToModeledElement view))

def readModelFieldIsDeclared (readModelName : String) (fieldName : String) : Bool := sliceReadModelDefinitions.any (fun readModel => readModel.name == readModelName && readModel.fields.any (fun readModelField => readModelField.name == fieldName))

def viewFieldSourceReadModelFieldResolves (field : ViewField) : Bool := field.sourceKind != "read_model" || readModelFieldIsDeclared field.sourceReadModel field.sourceField

def readModelFieldHasOriginalProvenance (field : ReadModelField) : Bool := (field.sourceKind == "event_attribute" && readModelFieldEventAttributeSourceResolves field) || field.sourceKind == "derivation" || field.sourceKind == "absence_default"

def viewFieldTracesToOriginalProvenance (field : ViewField) : Bool := field.sourceKind == "read_model" && sliceReadModelDefinitions.any (fun readModel => readModel.name == field.sourceReadModel && readModel.fields.any (fun readModelField => readModelField.name == field.sourceField && readModelFieldHasOriginalProvenance readModelField))

def viewFieldReadModelFieldSourcesResolve : Bool := sliceViewDefinitions.all (fun view => view.fields.all viewFieldSourceReadModelFieldResolves)

def displayedDataTraceToOriginalProvenance : Bool := sliceViewDefinitions.all (fun view => view.fields.all viewFieldTracesToOriginalProvenance)

def viewFieldHasBitLevelFlow (view : ViewDefinition) (field : ViewField) : Bool := bitLevelFlowCoversTarget view.name field.name

def commandInputDataFlowsAreComplete : Bool := sliceCommandDefinitions.all (fun command => command.inputs.all (commandInputHasBitLevelFlow command))

def eventAttributeDataFlowsAreComplete : Bool := sliceEventDefinitions.all (fun event => event.attributes.all (eventAttributeHasBitLevelFlow event))

def readModelFieldDataFlowsAreComplete : Bool := sliceReadModelDefinitions.all (fun readModel => readModel.fields.all (readModelFieldHasBitLevelFlow readModel))

def viewFieldDataFlowsAreComplete : Bool := sliceViewDefinitions.all (fun view => view.fields.all (viewFieldHasBitLevelFlow view))

def externalPayloadFieldDataFlowsAreComplete : Bool := sliceExternalPayloads.all (fun payload => payload.fields.all (externalPayloadFieldHasBitLevelFlow payload))

def modeledDataFlowsAreBitComplete : Bool := commandInputDataFlowsAreComplete && eventAttributeDataFlowsAreComplete && readModelFieldDataFlowsAreComplete && viewFieldDataFlowsAreComplete && externalPayloadFieldDataFlowsAreComplete

def controlInputHasAllowedSource (input : ControlInputProvision) : Bool := allowedControlInputSourceKinds.contains input.sourceKind

def controlInputHasProvenance (input : ControlInputProvision) : Bool := input.name.isEmpty == false && input.sourceDescription.isEmpty == false

def controlInputHasDescription (input : ControlInputProvision) : Bool := input.sourceDescription.isEmpty == false

def controlInputSessionInputHasDescription (input : ControlInputProvision) : Bool := input.sourceKind != CommandInputSourceKind.session || input.sourceDescription.isEmpty == false

def controlInputVisibilityIsModeled (input : ControlInputProvision) : Bool := (input.sourceKind != CommandInputSourceKind.actor || input.sketchToken.isEmpty == false || input.visibleToActor) && (input.decisionField == false || input.sketchToken.isEmpty == false || input.visibleToActor)

def controlInputDecisionFieldIsVisible (input : ControlInputProvision) : Bool := input.decisionField == false || input.sketchToken.isEmpty == false || input.visibleToActor

def controlInputActorInputIsVisible (input : ControlInputProvision) : Bool := input.sourceKind != CommandInputSourceKind.actor || input.sketchToken.isEmpty == false || input.visibleToActor

def controlHasSketchToken (control : ControlDefinition) : Bool := control.name.isEmpty == false && control.commandName.isEmpty == false && control.sketchToken.isEmpty == false

def controlReferencesKnownCommand (control : ControlDefinition) : Bool := sliceCommandNames.contains control.commandName || sliceReferencedCommandNames.contains control.commandName || sliceCommandDefinitions.any (fun command => command.name == control.commandName)

def controlProvidesCommandInput (control : ControlDefinition) (input : CommandInput) : Bool := control.inputs.any (fun providedInput => providedInput.name == input.name)

def controlProvidesEveryCommandInput (control : ControlDefinition) (command : CommandDefinition) : Bool := command.name != control.commandName || command.inputs.all (controlProvidesCommandInput control)

def commandErrorsHandledByControl (control : ControlDefinition) (command : CommandDefinition) : Bool := command.name != control.commandName || command.errors.all (fun error => control.handledErrors.contains error.name && control.recoveryBehavior.isEmpty == false)

def controlRecoveryBehaviorIsModeled (control : ControlDefinition) : Bool := control.handledErrors.isEmpty || allowedRecoveryKinds.contains control.recoveryBehavior

def navigationTargetTypeIsModeled (target : NavigationTarget) : Bool := target.targetType.isEmpty || allowedNavigationTargetTypes.contains target.targetType

def navigationTargetHasPayload (target : NavigationTarget) : Bool := target.targetName.isEmpty == false || target.externalWorkflowName.isEmpty == false || target.externalSystemName.isEmpty == false || target.handoffContract.isEmpty == false

def navigationControlDeclaresType (target : NavigationTarget) : Bool := navigationTargetHasPayload target == false || target.targetType.isEmpty == false

def navigationModeledViewTargetsExistingView (target : NavigationTarget) : Bool := target.targetType != "modeled_view" || (target.targetName.isEmpty == false && sliceViewNames.contains target.targetName)

def localViewStateNavigationTargetResolves (view : ViewDefinition) (target : NavigationTarget) : Bool := target.targetType != "local_view_state" || (target.targetName.isEmpty == false && (view.localStates.contains target.targetName || view.filters.contains target.targetName))

def navigationExternalWorkflowTargetsNamed (target : NavigationTarget) : Bool := target.targetType != "external_workflow" || target.externalWorkflowName.isEmpty == false

def navigationExternalSystemTargetsHaveContracts (target : NavigationTarget) : Bool := target.targetType != "external_system" || (target.externalSystemName.isEmpty == false && target.handoffContract.isEmpty == false)

def navigationTargetIsComplete (view : ViewDefinition) (target : NavigationTarget) : Bool := (target.targetType.isEmpty && target.targetName.isEmpty && target.externalWorkflowName.isEmpty && target.externalSystemName.isEmpty && target.handoffContract.isEmpty) || (target.targetType == "modeled_view" && target.targetName.isEmpty == false && sliceViewNames.contains target.targetName) || (target.targetType == "local_view_state" && localViewStateNavigationTargetResolves view target) || (target.targetType == "external_workflow" && navigationExternalWorkflowTargetsNamed target) || (target.targetType == "external_system" && navigationExternalSystemTargetsHaveContracts target)

def viewControlsHaveSketchTokens : Bool := sliceViewDefinitions.all (fun view => view.controls.all controlHasSketchToken)

def viewControlsReferenceKnownCommands : Bool := sliceViewDefinitions.all (fun view => view.controls.all controlReferencesKnownCommand)

def controlAppearsInSketch (view : ViewDefinition) (control : ControlDefinition) : Bool := control.sketchToken.isEmpty == false && view.sketchTokens.contains control.sketchToken

def viewControlsAppearInSketch : Bool := sliceViewDefinitions.all (fun view => view.controls.all (controlAppearsInSketch view))

def viewControlsProvideCommandInputs : Bool := sliceViewDefinitions.all (fun view => view.controls.all (fun control => sliceCommandDefinitions.all (controlProvidesEveryCommandInput control)))

def viewControlInputsHaveAllowedSources : Bool := sliceViewDefinitions.all (fun view => view.controls.all (fun control => control.inputs.all controlInputHasAllowedSource))

def viewControlInputsHaveProvenance : Bool := sliceViewDefinitions.all (fun view => view.controls.all (fun control => control.inputs.all controlInputHasProvenance))

def viewControlInputsHaveDescriptions : Bool := sliceViewDefinitions.all (fun view => view.controls.all (fun control => control.inputs.all controlInputHasDescription))

def viewControlSessionInputsHaveDescriptions : Bool := sliceViewDefinitions.all (fun view => view.controls.all (fun control => control.inputs.all controlInputSessionInputHasDescription))

def viewControlInputVisibilityIsModeled : Bool := sliceViewDefinitions.all (fun view => view.controls.all (fun control => control.inputs.all controlInputVisibilityIsModeled))

def viewControlDecisionFieldsAreVisible : Bool := sliceViewDefinitions.all (fun view => view.controls.all (fun control => control.inputs.all controlInputDecisionFieldIsVisible))

def viewControlActorInputsAreVisible : Bool := sliceViewDefinitions.all (fun view => view.controls.all (fun control => control.inputs.all controlInputActorInputIsVisible))

def viewControlsHandleCommandErrors : Bool := sliceViewDefinitions.all (fun view => view.controls.all (fun control => sliceCommandDefinitions.all (commandErrorsHandledByControl control)))

def viewControlRecoveryBehaviorIsModeled : Bool := sliceViewDefinitions.all (fun view => view.controls.all controlRecoveryBehaviorIsModeled)

def stateViewSlicesDoNotOwnCommands : Bool := sliceKind != SliceKindName.stateView || (sliceCommands.isEmpty && sliceCommandDefinitions.isEmpty)

def stateViewSlicesOwnViews : Bool := sliceKind != SliceKindName.stateView || (sliceViews.isEmpty == false || sliceViewDefinitions.isEmpty == false)

def stateViewSlicesOwnReadModels : Bool := sliceKind != SliceKindName.stateView || (sliceReadModels.isEmpty == false || sliceReadModelDefinitions.isEmpty == false)

def readModelOwnsProjectionPath (readModel : ReadModelDefinition) : Bool := readModel.fields.isEmpty == false && readModel.fields.all readModelFieldSourceIsComplete

def stateViewSlicesOwnProjectionPaths : Bool := sliceKind != SliceKindName.stateView || sliceReadModelDefinitions.all readModelOwnsProjectionPath

def stateViewSlicesRepresentSingleViewProjectionBoundary : Bool := sliceKind != SliceKindName.stateView || (sliceViewDefinitions.length == 1 && sliceReadModelDefinitions.isEmpty == false)

def stateChangeSlicesOwnCommands : Bool := sliceKind != SliceKindName.stateChange || (sliceCommands.isEmpty == false || sliceCommandDefinitions.isEmpty == false)

def stateChangeSlicesOwnEvents : Bool := sliceKind != SliceKindName.stateChange || (sliceEvents.isEmpty == false || sliceEventDefinitions.isEmpty == false)

def stateChangeSlicesOwnOutcomes : Bool := sliceKind != SliceKindName.stateChange || sliceOutcomeDefinitions.isEmpty == false

def stateChangeSlicesOwnErrors : Bool := sliceKind != SliceKindName.stateChange || commandErrorsAreDeclared

def stateChangeSlicesDoNotOwnReadModelsOrViews : Bool := sliceKind != SliceKindName.stateChange || (sliceReadModels.isEmpty && sliceReadModelDefinitions.isEmpty && sliceViews.isEmpty && sliceViewDefinitions.isEmpty)

def stateChangeSlicesDoNotOwnAutomationsOrTranslations : Bool := sliceKind != SliceKindName.stateChange || (sliceAutomations.isEmpty && sliceTranslations.isEmpty)

def stateChangeSlicesDoNotOwnControlsOrSketches : Bool := sliceKind != SliceKindName.stateChange || sliceViewDefinitions.all (fun view => view.controls.isEmpty && view.sketchTokens.isEmpty)

def translationSlicesDoNotOwnViews : Bool := sliceKind != SliceKindName.translation || (sliceViews.isEmpty && sliceViewDefinitions.isEmpty)

def recognizedSliceKind : Bool := true

def sliceRepresentsOneCoherentModelUnit : Bool := recognizedSliceKind && stateViewSlicesDoNotOwnCommands && stateViewSlicesOwnViews && stateViewSlicesOwnReadModels && stateViewSlicesOwnProjectionPaths && stateChangeSlicesOwnCommands && stateChangeSlicesOwnEvents && stateChangeSlicesOwnOutcomes && stateChangeSlicesOwnErrors && stateChangeSlicesDoNotOwnReadModelsOrViews && stateChangeSlicesDoNotOwnAutomationsOrTranslations && stateChangeSlicesDoNotOwnControlsOrSketches && translationSlicesDeclareExternalContracts && externalBoundariesHavePayloadContractsAndFieldProvenance && translationsTargetKnownCommands && translationsReferenceObservedExternalEvents && translationSlicesDoNotOwnViews && automationSlicesDeclareTriggers && automationSlicesRepresentOneReaction && automationsIssueKnownCommands && automationsHandleCommandErrors

def stateChangeSlicesRepresentSingleCommandBoundary : Bool := sliceKind != SliceKindName.stateChange || sliceCommandDefinitions.length == 1

def sliceRepresentsSmallestUsefulBehaviorBoundary : Bool := sliceRepresentsOneCoherentModelUnit && stateViewSlicesRepresentSingleViewProjectionBoundary && stateChangeSlicesRepresentSingleCommandBoundary && automationSlicesRepresentOneReaction && translationSlicesDeclareExternalContracts

def viewControlNavigationTypesAreModeled : Bool := sliceViewDefinitions.all (fun view => view.controls.all (fun control => navigationTargetTypeIsModeled control.navigation))

def viewControlNavigationTypesAreDeclared : Bool := sliceViewDefinitions.all (fun view => view.controls.all (fun control => navigationControlDeclaresType control.navigation))

def viewControlModeledViewNavigationTargetsResolve : Bool := sliceViewDefinitions.all (fun view => view.controls.all (fun control => navigationModeledViewTargetsExistingView control.navigation))

def viewControlExternalWorkflowNavigationTargetsNamed : Bool := sliceViewDefinitions.all (fun view => view.controls.all (fun control => navigationExternalWorkflowTargetsNamed control.navigation))

def viewControlExternalSystemNavigationTargetsHaveContracts : Bool := sliceViewDefinitions.all (fun view => view.controls.all (fun control => navigationExternalSystemTargetsHaveContracts control.navigation))

def viewControlNavigationTargetsAreComplete : Bool := sliceViewDefinitions.all (fun view => view.controls.all (fun control => navigationTargetIsComplete view control.navigation))

def sliceHasLocallyEmittedEvent : Bool := sliceEvents.isEmpty == false || sliceEventDefinitions.any (fun event => event.observed == false && event.shared == false)

def sliceStateChangeRequiresEvent : Prop := sliceKind = SliceKindName.stateChange -> sliceHasLocallyEmittedEvent

theorem sliceIdentityIsStable : sliceName = "Load State from Snapshot" := rfl

theorem sliceStateChangeRequiresEventIsStable : sliceStateChangeRequiresEvent := by
  simp [sliceStateChangeRequiresEvent, sliceHasLocallyEmittedEvent, sliceKind, sliceEvents, sliceEventDefinitions]

theorem sliceBitLevelDataFlowsAreStructured : sliceBitLevelDataFlows.all (fun flow => flow.datum.isEmpty == false && flow.sourceKind.isEmpty == false && flow.source.isEmpty == false && flow.transformationSemantics.isEmpty == false && flow.target.isEmpty == false && flow.bitEncoding.isEmpty == false) = true := rfl

theorem modeledDataFlowsAreBitCompleteIsStable : modeledDataFlowsAreBitComplete = true := rfl

theorem sliceScenariosHaveGwtIsStable : sliceScenariosHaveGwt = true := rfl

theorem sliceScenarioNamesAreUniqueIsStable : sliceScenarioNamesAreUnique = true := rfl

theorem sliceNamedDefinitionsAreUniquelyOwnedIsStable : sliceNamedDefinitionsAreUniquelyOwned = true := by
  native_decide

theorem sliceScenarioStreamsResolveIsStable : sliceScenarioStreamsResolve = true := rfl

theorem stateChangeScenariosNameStreamsIsStable : stateChangeScenariosNameStreams = true := by
  native_decide

theorem acceptanceScenariosAreUserFacingIsStable : acceptanceScenariosAreUserFacing = true := rfl

theorem stateViewReadModelsHaveProjectorContractsIsStable : stateViewReadModelsHaveProjectorContracts = true := rfl

theorem contractScenariosTargetKnownDefinitionsIsStable : contractScenariosTargetKnownDefinitions = true := rfl

theorem contractScenariosCoverModeledContractsIsStable : contractScenariosCoverModeledContracts = true := rfl

theorem commandInputsHaveAllowedSourcesIsStable : commandInputsHaveAllowedSources = true := rfl

theorem commandInputsHaveProvenanceIsStable : commandInputsHaveProvenance = true := rfl

theorem commandInputsWithoutIssuingControlsHaveProvenanceIsStable : commandInputsWithoutIssuingControlsHaveProvenance = true := rfl

theorem commandSessionInputsHaveDescriptionsIsStable : commandSessionInputsHaveDescriptions = true := rfl

theorem commandInputsTraceToInvocationSourcesIsStable : commandInputsTraceToInvocationSources = true := rfl

theorem commandInputsSourcedFromEventStreamsResolveIsStable : commandInputsSourcedFromEventStreamsResolve = true := rfl

theorem commandInputsSourcedFromExternalPayloadsResolveIsStable : commandInputsSourcedFromExternalPayloadsResolve = true := rfl

theorem commandInputsSourcedFromGeneratedValuesHaveCoordinatesIsStable : commandInputsSourcedFromGeneratedValuesHaveCoordinates = true := rfl

theorem commandInputsSourcedFromSessionValuesHaveCoordinatesIsStable : commandInputsSourcedFromSessionValuesHaveCoordinates = true := rfl

theorem commandInputsSourcedFromInvocationArgumentsHaveCoordinatesIsStable : commandInputsSourcedFromInvocationArgumentsHaveCoordinates = true := rfl

theorem commandErrorsAreDeclaredIsStable : commandErrorsAreDeclared = true := rfl

theorem commandErrorsHaveAllowedRecoveryIsStable : commandErrorsHaveAllowedRecovery = true := rfl

theorem commandErrorsHaveScenarioCoverageIsStable : commandErrorsHaveScenarioCoverage = true := rfl

theorem scenarioErrorReferencesAreDeclaredIsStable : scenarioErrorReferencesAreDeclared = true := rfl

theorem singletonCommandsDeclareRepeatBehaviorIsStable : singletonCommandsDeclareRepeatBehavior = true := rfl

theorem automationSlicesDeclareTriggersIsStable : automationSlicesDeclareTriggers = true := rfl

theorem automationSlicesRepresentOneReactionIsStable : automationSlicesRepresentOneReaction = true := rfl

theorem automationsIssueKnownCommandsIsStable : automationsIssueKnownCommands = true := rfl

theorem automationsHandleCommandErrorsIsStable : automationsHandleCommandErrors = true := rfl

theorem translationSlicesDeclareExternalContractsIsStable : translationSlicesDeclareExternalContracts = true := rfl

theorem externalBoundariesHavePayloadContractsAndFieldProvenanceIsStable : externalBoundariesHavePayloadContractsAndFieldProvenance = true := rfl

theorem translationsTargetKnownCommandsIsStable : translationsTargetKnownCommands = true := rfl

theorem translationsReferenceObservedExternalEventsIsStable : translationsReferenceObservedExternalEvents = true := rfl

theorem boardLanesAreCanonicalIsStable : boardLanesAreCanonical = true := rfl

theorem boardElementsUseCanonicalLanesIsStable : boardElementsUseCanonicalLanes = true := rfl

theorem boardElementsReferenceDeclarationsIsStable : boardElementsReferenceDeclarations = true := rfl

theorem automationBoardElementsAreDeclaredAutomationsIsStable : automationBoardElementsAreDeclaredAutomations = true := rfl

theorem externalBoardElementsAreObservedEventsIsStable : externalBoardElementsAreObservedEvents = true := rfl

theorem commandEventBoardEdgesMatchEmissionsIsStable : commandEventBoardEdgesMatchEmissions = true := rfl

theorem eventReadModelBoardEdgesMatchProjectionSourcesIsStable : eventReadModelBoardEdgesMatchProjectionSources = true := rfl

theorem viewCommandBoardEdgesMatchControlsIsStable : viewCommandBoardEdgesMatchControls = true := rfl

theorem boardConnectionsHaveCausalSemanticsIsStable : boardConnectionsHaveCausalSemantics = true := rfl

theorem externalEventTriggersMatchTranslationsIsStable : externalEventTriggersMatchTranslations = true := rfl

theorem externalEventsDoNotUpdateReadModelsIsStable : externalEventsDoNotUpdateReadModels = true := rfl

theorem readModelsFeedingViewsHaveIncomingEventUpdatesIsStable : readModelsFeedingViewsHaveIncomingEventUpdates = true := rfl

theorem commandsHaveIncomingTriggersIsStable : commandsHaveIncomingTriggers = true := rfl

theorem mainPathBoardHasNoDisconnectedIslandsIsStable : mainPathBoardHasNoDisconnectedIslands = true := rfl

theorem outcomeLabelsAreUniqueIsStable : outcomeLabelsAreUnique = true := rfl

theorem outcomeEventSetsAreNonEmptyIsStable : outcomeEventSetsAreNonEmpty = true := rfl

theorem outcomeEventSetsAreDistinctIsStable : outcomeEventSetsAreDistinct = true := rfl

theorem outcomeEventsAreKnownToSliceIsStable : outcomeEventsAreKnownToSlice = true := rfl

theorem eventsReferenceKnownStreamsIsStable : eventsReferenceKnownStreams = true := rfl

theorem commandEmittedEventsAreKnownIsStable : commandEmittedEventsAreKnown = true := rfl

theorem locallyEmittedEventsAreProducedByCommandsIsStable : locallyEmittedEventsAreProducedByCommands = true := rfl

theorem externalPayloadFieldsHaveProvenanceIsStable : externalPayloadFieldsHaveProvenance = true := rfl

theorem eventAttributesHaveAllowedSourcesIsStable : eventAttributesHaveAllowedSources = true := rfl

theorem eventAttributesHaveProvenanceIsStable : eventAttributesHaveProvenance = true := rfl

theorem eventAttributeSourcesAreCompleteIsStable : eventAttributeSourcesAreComplete = true := rfl

theorem storedEventFactsTraceToOriginalSourcesIsStable : storedEventFactsTraceToOriginalSources = true := rfl

theorem readModelFieldsHaveAllowedSourcesIsStable : readModelFieldsHaveAllowedSources = true := rfl

theorem readModelFieldsHaveProvenanceIsStable : readModelFieldsHaveProvenance = true := rfl

theorem readModelFieldSourcesAreCompleteIsStable : readModelFieldSourcesAreComplete = true := rfl

theorem readModelFieldEventAttributeSourcesResolveIsStable : readModelFieldEventAttributeSourcesResolve = true := rfl

theorem derivedReadModelFieldsHaveScenarioCoverageIsStable : derivedReadModelFieldsHaveScenarioCoverage = true := rfl

theorem absenceReadModelFieldsHaveScenarioCoverageIsStable : absenceReadModelFieldsHaveScenarioCoverage = true := rfl

theorem transitiveReadModelsHaveSemanticsIsStable : transitiveReadModelsHaveSemantics = true := rfl

theorem viewFieldReadModelFieldSourcesResolveIsStable : viewFieldReadModelFieldSourcesResolve = true := rfl

theorem displayedDataTraceToOriginalProvenanceIsStable : displayedDataTraceToOriginalProvenance = true := rfl

theorem viewFieldsHaveAllowedSourcesIsStable : viewFieldsHaveAllowedSources = true := rfl

theorem viewFieldsHaveProvenanceIsStable : viewFieldsHaveProvenance = true := rfl

theorem viewFieldSourcesAreCompleteIsStable : viewFieldSourcesAreComplete = true := rfl

theorem viewFieldsSourceFromUsedReadModelsIsStable : viewFieldsSourceFromUsedReadModels = true := rfl

theorem viewsHaveInformationSketchesIsStable : viewsHaveInformationSketches = true := rfl

theorem viewFieldsAppearInSketchIsStable : viewFieldsAppearInSketch = true := rfl

theorem viewSketchTokensMapToModeledElementsIsStable : viewSketchTokensMapToModeledElements = true := rfl

theorem viewControlsHaveSketchTokensIsStable : viewControlsHaveSketchTokens = true := rfl

theorem viewControlsReferenceKnownCommandsIsStable : viewControlsReferenceKnownCommands = true := rfl

theorem viewControlsAppearInSketchIsStable : viewControlsAppearInSketch = true := rfl

theorem viewControlsProvideCommandInputsIsStable : viewControlsProvideCommandInputs = true := rfl

theorem viewControlInputsHaveAllowedSourcesIsStable : viewControlInputsHaveAllowedSources = true := rfl

theorem viewControlInputsHaveProvenanceIsStable : viewControlInputsHaveProvenance = true := rfl

theorem viewControlInputsHaveDescriptionsIsStable : viewControlInputsHaveDescriptions = true := rfl

theorem viewControlSessionInputsHaveDescriptionsIsStable : viewControlSessionInputsHaveDescriptions = true := rfl

theorem viewControlInputVisibilityIsModeledIsStable : viewControlInputVisibilityIsModeled = true := rfl

theorem viewControlDecisionFieldsAreVisibleIsStable : viewControlDecisionFieldsAreVisible = true := rfl

theorem viewControlActorInputsAreVisibleIsStable : viewControlActorInputsAreVisible = true := rfl

theorem viewControlsHandleCommandErrorsIsStable : viewControlsHandleCommandErrors = true := rfl

theorem viewControlRecoveryBehaviorIsModeledIsStable : viewControlRecoveryBehaviorIsModeled = true := rfl

theorem stateViewSlicesDoNotOwnCommandsIsStable : stateViewSlicesDoNotOwnCommands = true := rfl

theorem stateViewSlicesOwnViewsIsStable : stateViewSlicesOwnViews = true := rfl

theorem stateViewSlicesOwnReadModelsIsStable : stateViewSlicesOwnReadModels = true := rfl

theorem stateViewSlicesOwnProjectionPathsIsStable : stateViewSlicesOwnProjectionPaths = true := rfl

theorem stateViewSlicesRepresentSingleViewProjectionBoundaryIsStable : stateViewSlicesRepresentSingleViewProjectionBoundary = true := by
  native_decide

theorem stateChangeSlicesOwnCommandsIsStable : stateChangeSlicesOwnCommands = true := rfl

theorem stateChangeSlicesOwnEventsIsStable : stateChangeSlicesOwnEvents = true := by
  native_decide

theorem stateChangeSlicesOwnOutcomesIsStable : stateChangeSlicesOwnOutcomes = true := by
  native_decide

theorem stateChangeSlicesOwnErrorsIsStable : stateChangeSlicesOwnErrors = true := by
  native_decide

theorem stateChangeSlicesDoNotOwnReadModelsOrViewsIsStable : stateChangeSlicesDoNotOwnReadModelsOrViews = true := rfl

theorem stateChangeSlicesDoNotOwnAutomationsOrTranslationsIsStable : stateChangeSlicesDoNotOwnAutomationsOrTranslations = true := rfl

theorem stateChangeSlicesDoNotOwnControlsOrSketchesIsStable : stateChangeSlicesDoNotOwnControlsOrSketches = true := rfl

theorem translationSlicesDoNotOwnViewsIsStable : translationSlicesDoNotOwnViews = true := rfl

theorem sliceRepresentsOneCoherentModelUnitIsStable : sliceRepresentsOneCoherentModelUnit = true := by
  native_decide

theorem stateChangeSlicesRepresentSingleCommandBoundaryIsStable : stateChangeSlicesRepresentSingleCommandBoundary = true := by
  native_decide

theorem sliceRepresentsSmallestUsefulBehaviorBoundaryIsStable : sliceRepresentsSmallestUsefulBehaviorBoundary = true := by
  native_decide

theorem viewControlNavigationTypesAreModeledIsStable : viewControlNavigationTypesAreModeled = true := rfl

theorem viewControlNavigationTypesAreDeclaredIsStable : viewControlNavigationTypesAreDeclared = true := rfl

theorem viewControlModeledViewNavigationTargetsResolveIsStable : viewControlModeledViewNavigationTargetsResolve = true := rfl

theorem viewControlExternalWorkflowNavigationTargetsNamedIsStable : viewControlExternalWorkflowNavigationTargetsNamed = true := rfl

theorem viewControlExternalSystemNavigationTargetsHaveContractsIsStable : viewControlExternalSystemNavigationTargetsHaveContracts = true := rfl

theorem viewControlNavigationTargetsAreCompleteIsStable : viewControlNavigationTargetsAreComplete = true := rfl

end LoadStateFromSnapshot
