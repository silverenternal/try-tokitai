const translations = {
  zh: {
    newSession: "+ \u65b0\u5efa\u4f1a\u8bdd",
    sessionsLabel: "\u6700\u8fd1\u4f1a\u8bdd",
    runtimeOnline: "Gateway",
    composerPlaceholder: "\u8f93\u5165\u4efb\u52a1\u540e\u6309 Enter \u53d1\u9001\uff0cShift+Enter \u6362\u884c\u3002",
    fieldPrimaryModel: "\u4e3b\u6a21\u578b",
    fieldCompetitionMode: "\u7ade\u8d5b\u6a21\u5f0f",
    fieldPrivacyMode: "\u9690\u79c1\u4fdd\u62a4",
    fieldApproveSafe: "\u542f\u7528\u81ea\u52a8\u6279\u51c6",
    fieldRiskBoundary: "\u98ce\u9669\u8fb9\u754c",
    fieldMaxCalls: "\u6bcf\u5206\u949f\u6700\u5927\u5de5\u5177\u8c03\u7528",
    fieldBurstLimit: "\u7a81\u53d1\u4e0a\u9650",
    riskSafe: "Safe",
    riskModerate: "Moderate",
    riskUnlimited: "Unlimited",
    limitUnlimited: "Unlimited",
    helpCompetitionMode: "\u5728\u5173\u952e\u6b65\u9aa4\u6682\u505c\uff0c\u7b49\u5f85\u4eba\u5de5\u786e\u8ba4\u540e\u7ee7\u7eed\u3002",
    helpPrivacyMode: "\u9650\u5236\u654f\u611f\u5185\u5bb9\u66b4\u9732\uff0c\u5e76\u52a0\u5f3a\u5bf9\u5916\u90e8\u5de5\u5177\u4e0e\u6570\u636e\u8bfb\u53d6\u7684\u7ea6\u675f\u3002",
    helpApproveSafe: "\u5173\u95ed\u65f6\uff0c\u65e0\u8bba\u98ce\u9669\u8fb9\u754c\u5982\u4f55\uff0c\u6240\u6709\u5de5\u5177\u8c03\u7528\u90fd\u9700\u8981\u4eba\u5de5\u6279\u51c6\u3002\u5f00\u542f\u540e\uff0c\u624d\u4f1a\u6309\u4e0b\u65b9\u98ce\u9669\u8fb9\u754c\u81ea\u52a8\u653e\u884c\u3002",
    helpRiskBoundary: "\u63a7\u5236\u81ea\u52a8\u6279\u51c6\u5de5\u5177\u8c03\u7528\u65f6\u5141\u8bb8\u8fbe\u5230\u7684\u6700\u9ad8\u98ce\u9669\u7b49\u7ea7\u3002",
    helpRateLimit: "\u7528\u4e8e\u9650\u5236\u5de5\u5177\u8c03\u7528\u901f\u7387\uff0c\u907f\u514d\u8fc7\u5feb\u89e6\u53d1\u5927\u91cf\u5916\u90e8\u64cd\u4f5c\u3002",
    fieldWorkspaceRoot: "\u5de5\u4f5c\u533a\u8def\u5f84",
    fieldApiKey: "API Key",
    emptyState: "\u4eca\u5929\u60f3\u7814\u7a76\u70b9\u4ec0\u4e48\uff1f",
    toastSessionCreated: "\u5df2\u65b0\u5efa\u4f1a\u8bdd",
    toastSessionSwitched: "\u5df2\u5207\u6362\u4f1a\u8bdd",
    toastSendFailed: "\u53d1\u9001\u5931\u8d25\uff0c\u8bf7\u7a0d\u540e\u91cd\u8bd5\u3002",
    toastSettingsSaved: "\u8bbe\u7f6e\u5df2\u5e94\u7528",
    toastWorkspaceSwitched: "\u5df2\u5207\u6362\u5de5\u4f5c\u6587\u4ef6\u5939",
    toastWorkspaceCanceled: "\u5df2\u53d6\u6d88\u5de5\u4f5c\u6587\u4ef6\u5939\u9009\u62e9",
    toastSandboxInitialized: "\u5df2\u5b8c\u6210\u9996\u6b21\u6c99\u7bb1\u521d\u59cb\u5316\uff0c\u9ed8\u8ba4\u5de5\u4f5c\u533a\u5df2\u5207\u6362\u5230\u9694\u79bb workspace\u3002",
    providerReady: "\u53ef\u7528",
    providerPrimary: "\u4e3b\u7528",
    providerAvailable: "\u5df2\u63a5\u5165",
    effortTitle: "Effort",
    effortFast: "\u66f4\u5feb",
    effortSmart: "\u66f4\u5f3a",
    effortLow: "Low",
    effortMedium: "Medium",
    effortHigh: "High",
    effortMax: "Max",
    effortMetaLow: "\u6e29\u5ea6 0.60 / \u6700\u5927\u8f93\u51fa\u7ea6\u4e3a\u6a21\u578b\u4e0a\u9650\u7684 35%",
    effortMetaMedium: "\u6e29\u5ea6 0.70 / \u6700\u5927\u8f93\u51fa\u7ea6\u4e3a\u6a21\u578b\u4e0a\u9650\u7684 55%",
    effortMetaHigh: "\u6e29\u5ea6 0.85 / \u6700\u5927\u8f93\u51fa\u7ea6\u4e3a\u6a21\u578b\u4e0a\u9650\u7684 80%",
    effortMetaMax: "\u6e29\u5ea6 0.90 / \u6700\u5927\u8f93\u51fa\u7ea6\u4e3a\u6a21\u578b\u4e0a\u9650\u7684 100%",
    effortMetaDeepThink: "\u82e5\u540c\u65f6\u5f00\u542f\u6df1\u5ea6\u601d\u8003\uff0c\u6700\u5927\u8f93\u51fa\u4f1a\u76f4\u63a5\u63d0\u5347\u5230\u6a21\u578b\u4e0a\u9650\u3002",
    settingsLabel: "\u8bbe\u7f6e",
    settingsSave: "\u4fdd\u5b58",
    tabModel: "Model",
    tabSecurity: "Security",
    tabRuntime: "Runtime",
    tabProviders: "Providers",
    currentSession: "\u5f53\u524d\u4f1a\u8bdd",
    sessionUntitled: "\u672a\u547d\u540d\u4f1a\u8bdd",
    renameAction: "\u91cd\u547d\u540d",
    deleteAction: "\u5220\u9664",
    renamePrompt: "\u91cd\u547d\u540d\u4f1a\u8bdd",
    deleteConfirm: "\u786e\u5b9a\u5220\u9664\u8be5\u4f1a\u8bdd\u5417\uff1f",
    languageLabel: "\u8bed\u8a00",
    researchLabel: "\u79d1\u7814\u6d41\u7a0b",
    researchTemplateTopic: "\u79d1\u7814\u5de5\u4f5c\u6d41\u6a21\u677f",
    researchStatus: "\u5f53\u524d\u9636\u6bb5",
    researchStatusTemplate: "\u6d41\u7a0b\u6a21\u677f",
    researchNext: "\u4e0b\u4e00\u9636\u6bb5",
    researchSecurity: "\u5b89\u5168\u7b49\u7ea7",
    researchWorkspace: "\u5de5\u4f5c\u533a",
    researchAssessment: "\u8bc4\u4f30",
    researchAssessmentGood: "\u5f53\u524d\u6d41\u7a0b\u6574\u4f53\u5408\u7406\uff0c\u5177\u5907\u6267\u884c\u57fa\u7840\u3002",
    researchAssessmentHuman: "\u6587\u732e\u8d28\u91cf\u3001\u5b9e\u9a8c\u6761\u4ef6\u3001\u9a8c\u8bc1\u4e0e\u8bc4\u5ba1\u4ecd\u9700\u4eba\u5de5\u628a\u5173\u3002",
    researchWaiting: "\u7b49\u5f85\u4eba\u5de5\u786e\u8ba4",
    researchCompetition: "\u7ade\u8d5b\u6a21\u5f0f",
    phaseLiterature: "\u6587\u732e\u7efc\u8ff0",
    phaseHypothesis: "\u5047\u8bbe\u751f\u6210",
    phaseExperiment: "\u5b9e\u9a8c\u8bbe\u8ba1",
    phaseExecution: "\u5b9e\u9a8c\u6267\u884c",
    phaseValidation: "\u7ed3\u679c\u9a8c\u8bc1",
    phasePaper: "\u8bba\u6587\u5199\u4f5c",
    phaseReview: "\u4eba\u5de5\u8bc4\u5ba1",
    noteLiterature: "\u68c0\u7d22\u76f8\u5173\u5de5\u4f5c\uff0c\u68b3\u7406\u65b9\u6cd5\u3001\u6570\u636e\u96c6\u4e0e\u7814\u7a76\u7a7a\u767d\u3002",
    noteHypothesis: "\u63d0\u51fa\u53ef\u68c0\u9a8c\u7684\u7814\u7a76\u5047\u8bbe\uff0c\u5e76\u6bd4\u8f83\u65b0\u9896\u6027\u4e0e\u53ef\u884c\u6027\u3002",
    noteExperiment: "\u660e\u786e\u6570\u636e\u3001\u57fa\u7ebf\u3001\u6307\u6807\u4e0e\u5b9e\u9a8c\u534f\u8bae\u3002",
    noteExecution: "\u5b9e\u73b0\u65b9\u6848\u3001\u8fd0\u884c\u5b9e\u9a8c\u5e76\u6c89\u6dc0\u7ed3\u679c\u3002",
    noteValidation: "\u5206\u6790\u7ed3\u679c\uff0c\u68c0\u67e5\u7edf\u8ba1\u663e\u8457\u6027\u4e0e\u8fb9\u754c\u6761\u4ef6\u3002",
    notePaper: "\u6574\u7406\u8bba\u6587\u7ed3\u6784\u3001\u65b9\u6cd5\u63cf\u8ff0\u4e0e\u5b9e\u9a8c\u7ed3\u8bba\u3002",
    noteReview: "\u8fdb\u884c\u4eba\u5de5\u5ba1\u9605\uff0c\u4fee\u6b63\u8fc7\u5ea6\u7ed3\u8bba\u4e0e\u590d\u73b0\u5b9e\u9a8c\u7ec6\u8282\u3002",
    activityReviewing: "\u6b63\u5728\u67e5\u770b",
    activityComposing: "\u6b63\u5728\u7ec4\u7ec7\u56de\u7b54",
    activityEditing: "\u6b63\u5728\u7f16\u8f91",
    reviewTitle: "\u5de5\u4f5c\u533a\u5ba1\u67e5",
    reviewMeta: "{files} \u4e2a\u6587\u4ef6\u53d8\u66f4 / +{additions} / -{deletions}",
    reviewOpen: "\u67e5\u770b diff",
    reviewLoading: "\u6b63\u5728\u52a0\u8f7d diff...",
    reviewEmpty: "\u5f53\u524d\u5de5\u4f5c\u533a\u6ca1\u6709\u53d8\u66f4\u3002",
    reviewUnavailable: "\u5f53\u524d\u5de5\u4f5c\u533a\u6682\u65f6\u65e0\u6cd5\u67e5\u770b\u53d8\u66f4\u3002",
    reviewError: "\u65e0\u6cd5\u52a0\u8f7d\u6587\u4ef6 diff\u3002",
    reviewerFeedbackTitle: "\u8bc4\u5ba1\u53cd\u9988",
    reviewerFeedbackMeta: "\u672a\u89e3\u51b3 {count} / \u603b\u8ba1 {total}",
    reviewerFeedbackCurrentRun: "\u5f53\u524d Run",
    reviewerFeedbackEmpty: "\u5f53\u524d\u8fd8\u6ca1\u6709\u8bc4\u5ba1\u53cd\u9988\u3002",
    reviewerFeedbackReviewer: "\u8bc4\u5ba1\u4eba",
    reviewerFeedbackScore: "\u8bc4\u5206",
    reviewerFeedbackComment: "\u610f\u89c1",
    reviewerFeedbackRunId: "Run ID",
    reviewerFeedbackAdd: "\u6dfb\u52a0\u53cd\u9988",
    reviewerFeedbackResolve: "\u6807\u8bb0\u5df2\u89e3\u51b3",
    reviewerFeedbackResolved: "\u5df2\u89e3\u51b3",
    reviewerFeedbackOpen: "\u5f85\u5904\u7406",
    reviewerFeedbackRefresh: "\u5237\u65b0",
    reviewerFeedbackDraftHint: "\u5c06 reviewer / \u4eba\u5ba1\u610f\u89c1\u7ed1\u5b9a\u5230\u5f53\u524d run\u3002",
    reviewerFeedbackScoreHint: "0-100\uff0c\u53ef\u7559\u7a7a",
    reviewerFeedbackValidation: "\u8bf7\u586b\u5199\u8bc4\u5ba1\u4eba\u548c\u610f\u89c1\u3002",
    reviewerFeedbackScoreInvalid: "\u8bc4\u5206\u9700\u8981\u5728 0 \u5230 100 \u4e4b\u95f4\u3002",
    toastReviewerFeedbackSaved: "\u8bc4\u5ba1\u53cd\u9988\u5df2\u8bb0\u5f55",
    toastReviewerFeedbackResolved: "\u8bc4\u5ba1\u53cd\u9988\u5df2\u6807\u8bb0\u4e3a\u5df2\u89e3\u51b3",
    toastReviewerFeedbackRefreshed: "\u8bc4\u5ba1\u53cd\u9988\u5df2\u5237\u65b0",
    paperWorkflowTitle: "\u8bba\u6587\u4ea7\u51fa",
    paperWorkflowRun: "\u751f\u6210\u8bba\u6587",
    paperWorkflowRunning: "\u6b63\u5728\u751f\u6210\u8bba\u6587...",
    paperWorkflowOpen: "\u6253\u5f00",
    paperWorkflowSummaryFallback: "\u5c06\u5f53\u524d\u7814\u7a76\u95ed\u73af\u6574\u7406\u4e3a\u8bba\u6587\u3001\u9644\u5f55\u4e0e\u7ed3\u679c\u5305\u3002",
    paperWorkflowEmpty: "\u5f53\u524d\u8fd8\u6ca1\u6709\u8bba\u6587\u4ea7\u7269\u3002",
    paperWorkflowArtifacts: "\u8bba\u6587\u4ea7\u7269",
    paperWorkflowPrimary: "\u4e3b\u7a3f",
    toastPaperWorkflowDone: "\u8bba\u6587\u4ea7\u7269\u5df2\u751f\u6210",
    paperWorkflowPromptTitle: "\u8bba\u6587\u5361\u7247",
    paperWorkflowPromptReady: "\u5f53\u524d\u7814\u7a76\u5bf9\u8bdd\u5df2\u5177\u5907\u751f\u6210\u8bba\u6587\u7684\u6761\u4ef6\u3002",
    paperWorkflowPromptGenerate: "\u751f\u6210\u8bba\u6587",
    paperWorkflowPromptLater: "\u6682\u4e0d\u751f\u6210",
    paperWorkflowPromptHint: "\u53ef\u5728\u5f53\u524d session \u7684\u7814\u7a76\u9762\u677f\u91cc\u7a0d\u540e\u518d\u751f\u6210\u3002",
    toastPaperWorkflowDismissed: "\u5df2\u5173\u95ed\u672c\u6b21\u8bba\u6587\u751f\u6210\u63d0\u793a",
    searchLabel: "\u641c\u7d22",
    searchTitle: "\u7814\u7a76\u68c0\u7d22",
    searchSubtitle: "Web via provider / Papers via official APIs / Datasets via retrieval base",
    searchModeWeb: "\u7f51\u9875",
    searchModePapers: "\u8bba\u6587",
    searchModeTracking: "\u8ffd\u8e2a",
    searchModeBenchmarks: "\u57fa\u51c6",
    searchModeModels: "Models",
    searchModeDatasets: "\u6570\u636e\u96c6",
    searchModeGitHub: "GitHub",
    searchPlaceholderWeb: "\u7528 Web provider \u641c\u7d22\u516c\u5f00\u7f51\u9875\u8d44\u6599",
    searchPlaceholderPapers: "\u641c\u7d22\u8bba\u6587\u4e3b\u9898\u3001\u65b9\u6cd5\u6216 benchmark",
    searchPlaceholderTracking: "\u641c\u7d22 Hugging Face Trending Papers \u7b49\u7814\u7a76\u8ffd\u8e2a\u9762",
    searchPlaceholderBenchmarks: "\u641c\u7d22 MLPerf \u5b98\u65b9 benchmark \u9762\uff0c\u4f8b\u5982 inference\u3001training\u3001storage",
    searchPlaceholderModels: "\u641c\u7d22 ONNX Model Zoo \u5b98\u65b9\u6a21\u578b\uff0c\u4f8b\u5982 bert\u3001resnet\u3001diffusion",
    searchPlaceholderDatasets: "\u76f4\u8fde OpenML / Hugging Face / Papers With Code / Kaggle \u641c\u7d22\u516c\u5f00\u6570\u636e\u96c6\u5019\u9009",
    searchPlaceholderGitHub: "\u641c\u7d22 GitHub \u4ed3\u5e93\u3001\u4ee3\u7801\u6216\u6570\u636e\u96c6\uff0c\u4f8b\u5982\uff1arepo:openai evals \u6216 mnist pytorch",
    searchRun: "\u641c\u7d22",
    searchOpen: "\u6253\u5f00",
    searchUseDataset: "\u751f\u6210 Manifest",
    searchHealthReady: "\u5df2\u8fde\u63a5",
    searchHealthDown: "\u672a\u8fde\u63a5",
    searchHealthDegraded: "\u90e8\u5206\u53ef\u7528",
    searchHealthUnknown: "\u672a\u77e5",
    searchEmpty: "\u8fd8\u6ca1\u6709\u68c0\u7d22\u7ed3\u679c\u3002",
    searchLoading: "\u6b63\u5728\u641c\u7d22...",
    searchError: "\u641c\u7d22\u5931\u8d25\u3002",
    searchDatasetHealth: "\u7814\u7a76\u68c0\u7d22\u5e95\u5ea7",
    searchWebHealth: "Web provider",
    searchPapersHealth: "\u5b98\u65b9\u8bba\u6587 API",
    searchTrackingHealth: "\u7814\u7a76\u8ffd\u8e2a",
    searchBenchmarksHealth: "\u57fa\u51c6\u5e73\u53f0",
    searchModelsHealth: "ONNX Model Zoo",
    searchGitHubHealth: "GitHub \u68c0\u7d22",
    searchManifestReady: "Manifest \u5df2\u751f\u6210",
    searchNoUrl: "\u5f53\u524d\u7ed3\u679c\u6ca1\u6709\u53ef\u7528\u94fe\u63a5",
    toastSearchCopied: "\u641c\u7d22\u7ed3\u679c\u5df2\u5237\u65b0",
    autoSkillsTitle: "\u672c\u8f6e\u81ea\u52a8\u542f\u7528\u7684 skills",
    autoSkillsKindWorkflow: "\u6d41\u7a0b",
    autoSkillsKindSubfield: "\u5b50\u9886\u57df",
    autoSkillsKindGeneral: "\u901a\u7528",
    modeChat: "Chat",
    modeResearch: "Agent",
    gitLabel: "Git \u5de5\u4f5c\u53f0",
    gitOverview: "\u6982\u89c8",
    gitChanges: "\u53d8\u66f4",
    gitHistory: "\u63d0\u4ea4",
    gitBranches: "\u5206\u652f",
    gitGraph: "Graph",
    gitRefresh: "\u5237\u65b0",
    gitFetch: "Fetch",
    gitPull: "Pull",
    gitPush: "Push",
    gitStageAll: "\u5168\u90e8\u6682\u5b58",
    gitUnstageAll: "\u5168\u90e8\u53d6\u6d88\u6682\u5b58",
    gitCommit: "\u63d0\u4ea4",
    gitCommitPrompt: "\u8f93\u5165\u63d0\u4ea4\u8bf4\u660e",
    gitUnavailable: "\u5f53\u524d\u5de5\u4f5c\u533a\u6682\u65f6\u65e0\u6cd5\u8bfb\u53d6 Git \u6570\u636e\u3002",
    gitWorkingTree: "\u5de5\u4f5c\u533a\u72b6\u6001",
    gitRepository: "\u4ed3\u5e93",
    gitBranchCurrent: "\u5f53\u524d\u5206\u652f",
    gitChangesSummary: "\u53d8\u66f4\u6982\u89c8",
    gitGraphEmpty: "\u5f53\u524d\u4ed3\u5e93\u8fd8\u6ca1\u6709\u53ef\u663e\u793a\u7684\u63d0\u4ea4\u56fe\u3002",
    gitHistoryEmpty: "\u5f53\u524d\u4ed3\u5e93\u8fd8\u6ca1\u6709\u63d0\u4ea4\u5386\u53f2\u3002",
    gitBranchesEmpty: "\u5f53\u524d\u4ed3\u5e93\u8fd8\u6ca1\u6709\u5206\u652f\u4fe1\u606f\u3002",
    gitDiffWorking: "\u5de5\u4f5c\u533a Diff",
    gitDiffStaged: "\u6682\u5b58\u533a Diff",
    gitNoDiff: "\u5f53\u524d\u6ca1\u6709\u53ef\u663e\u793a\u7684 diff\u3002",
    gitCommitAuthor: "\u4f5c\u8005",
    gitCheckout: "\u5207\u6362",
    gitCreateBranch: "\u65b0\u5efa\u5206\u652f",
    gitDeleteBranch: "\u5220\u9664\u5206\u652f",
    gitBranchPrompt: "\u8f93\u5165\u5206\u652f\u540d",
    gitDeleteBranchConfirm: "\u786e\u5b9a\u5220\u9664\u5206\u652f {branch} \u5417\uff1f",
    gitStage: "\u6682\u5b58",
    gitUnstage: "\u53d6\u6d88\u6682\u5b58",
    gitDiscard: "\u4e22\u5f03",
    gitAheadBehind: "\u9886\u5148 {ahead} / \u843d\u540e {behind}",
    gitClean: "\u5de5\u4f5c\u533a\u5e72\u51c0",
    gitStaged: "\u5df2\u6682\u5b58",
    gitModified: "\u5df2\u4fee\u6539",
    gitUntracked: "\u672a\u8ddf\u8e2a",
    gitConflicted: "\u51b2\u7a81",
    gitActionFailed: "Git \u64cd\u4f5c\u5931\u8d25\uff0c\u8bf7\u7a0d\u540e\u91cd\u8bd5\u3002",
    gitNoStatus: "\u5f53\u524d\u6ca1\u6709 Git \u72b6\u6001\u6458\u8981\u3002",
  },
  en: {
    newSession: "+ New session",
    sessionsLabel: "Recents",
    runtimeOnline: "Gateway",
    composerPlaceholder: "Press Enter to send. Shift+Enter for a new line.",
    fieldPrimaryModel: "Primary model",
    fieldCompetitionMode: "Competition mode",
    fieldPrivacyMode: "Privacy guard",
    fieldApproveSafe: "Enable auto approval",
    fieldRiskBoundary: "Risk boundary",
    fieldMaxCalls: "Max tool calls / min",
    fieldBurstLimit: "Burst limit",
    riskSafe: "Safe",
    riskModerate: "Moderate",
    riskUnlimited: "Unlimited",
    limitUnlimited: "Unlimited",
    helpCompetitionMode: "Pause at key checkpoints and wait for human approval before continuing.",
    helpPrivacyMode: "Reduce sensitive exposure and tighten rules around external tools and data access.",
    helpApproveSafe: "When off, every tool call still requires manual approval regardless of the risk boundary. When on, tool calls are auto-approved up to the selected boundary.",
    helpRiskBoundary: "Sets the highest risk level allowed for automatically approved tool calls.",
    helpRateLimit: "Limits tool-call velocity so the agent does not trigger many external actions too quickly.",
    fieldWorkspaceRoot: "Workspace root",
    fieldApiKey: "API Key",
    emptyState: "What would you like to explore today?",
    toastSessionCreated: "New session created",
    toastSessionSwitched: "Session switched",
    toastSendFailed: "Send failed. Please try again.",
    toastSettingsSaved: "Settings applied",
    toastWorkspaceSwitched: "Workspace switched",
    toastWorkspaceCanceled: "Workspace selection cancelled",
    toastSandboxInitialized: "Sandbox initialized on first launch. The default workspace is now isolated.",
    providerReady: "ready",
    providerPrimary: "primary",
    providerAvailable: "connected",
    effortTitle: "Effort",
    effortFast: "Faster",
    effortSmart: "Smarter",
    effortLow: "Low",
    effortMedium: "Medium",
    effortHigh: "High",
    effortMax: "Max",
    effortMetaLow: "Temperature 0.60 / max output about 35% of the model limit",
    effortMetaMedium: "Temperature 0.70 / max output about 55% of the model limit",
    effortMetaHigh: "Temperature 0.85 / max output about 80% of the model limit",
    effortMetaMax: "Temperature 0.90 / max output about 100% of the model limit",
    effortMetaDeepThink: "If deep thinking is enabled, max output rises to the full model limit.",
    settingsLabel: "Settings",
    settingsSave: "Save",
    tabModel: "Model",
    tabSecurity: "Security",
    tabRuntime: "Runtime",
    tabProviders: "Providers",
    currentSession: "Current session",
    sessionUntitled: "Untitled",
    renameAction: "Rename",
    deleteAction: "Delete",
    renamePrompt: "Rename session",
    deleteConfirm: "Delete this session?",
    languageLabel: "Language",
    researchLabel: "Research pipeline",
    researchTemplateTopic: "Scientific workflow template",
    researchStatus: "Current phase",
    researchStatusTemplate: "Workflow template",
    researchNext: "Next phase",
    researchSecurity: "Security level",
    researchWorkspace: "Workspace",
    researchAssessment: "Assessment",
    researchAssessmentGood: "The workflow is well-structured and operationally viable.",
    researchAssessmentHuman: "Literature quality, experimental conditions, validation, and review still need human oversight.",
    researchWaiting: "Awaiting approval",
    researchCompetition: "Competition mode",
    phaseLiterature: "Literature review",
    phaseHypothesis: "Hypothesis generation",
    phaseExperiment: "Experiment design",
    phaseExecution: "Execution",
    phaseValidation: "Validation",
    phasePaper: "Paper writing",
    phaseReview: "Review",
    noteLiterature: "Search prior work, datasets, methods, and open gaps.",
    noteHypothesis: "Propose testable ideas and rank them by novelty and feasibility.",
    noteExperiment: "Define data, baselines, metrics, and the evaluation protocol.",
    noteExecution: "Implement the plan, run experiments, and collect outputs.",
    noteValidation: "Analyze results and test whether the hypothesis holds.",
    notePaper: "Draft the paper structure, method, and experimental findings.",
    noteReview: "Human review checks claims, reproducibility, and weak spots.",
    activityReviewing: "Reviewing context",
    activityComposing: "Composing response",
    activityEditing: "Updating answer",
    reviewTitle: "Working tree review",
    reviewMeta: "{files} files changed / +{additions} / -{deletions}",
    reviewOpen: "Open diff",
    reviewLoading: "Loading diff...",
    reviewEmpty: "No working tree changes.",
    reviewUnavailable: "Review is unavailable for this workspace.",
    reviewError: "Unable to load file diff.",
    reviewerFeedbackTitle: "Reviewer feedback",
    reviewerFeedbackMeta: "{count} open / {total} total",
    reviewerFeedbackCurrentRun: "Current run",
    reviewerFeedbackEmpty: "No reviewer feedback yet.",
    reviewerFeedbackReviewer: "Reviewer",
    reviewerFeedbackScore: "Score",
    reviewerFeedbackComment: "Comment",
    reviewerFeedbackRunId: "Run ID",
    reviewerFeedbackAdd: "Add feedback",
    reviewerFeedbackResolve: "Resolve",
    reviewerFeedbackResolved: "Resolved",
    reviewerFeedbackOpen: "Open",
    reviewerFeedbackRefresh: "Refresh",
    reviewerFeedbackDraftHint: "Bind human-review notes to the current run.",
    reviewerFeedbackScoreHint: "0-100, optional",
    reviewerFeedbackValidation: "Reviewer and comment are required.",
    reviewerFeedbackScoreInvalid: "Score must be between 0 and 100.",
    toastReviewerFeedbackSaved: "Reviewer feedback recorded",
    toastReviewerFeedbackResolved: "Reviewer feedback resolved",
    toastReviewerFeedbackRefreshed: "Reviewer feedback refreshed",
    paperWorkflowTitle: "Paper output",
    paperWorkflowRun: "Generate paper",
    paperWorkflowRunning: "Generating paper...",
    paperWorkflowOpen: "Open",
    paperWorkflowSummaryFallback: "Assemble the current research loop into a paper, appendix, and result bundle.",
    paperWorkflowEmpty: "No paper artifacts yet.",
    paperWorkflowArtifacts: "Artifacts",
    paperWorkflowPrimary: "Primary draft",
    toastPaperWorkflowDone: "Paper artifacts generated",
    paperWorkflowPromptTitle: "Paper card",
    paperWorkflowPromptReady: "This research conversation is ready for an optional paper generation step.",
    paperWorkflowPromptGenerate: "Generate paper",
    paperWorkflowPromptLater: "Not now",
    paperWorkflowPromptHint: "You can still generate it later from the research panel for this session.",
    toastPaperWorkflowDismissed: "Paper generation prompt dismissed for this session",
    searchLabel: "Search",
    searchTitle: "Research Retrieval",
    searchSubtitle: "Web via provider / Papers via official APIs / Datasets via retrieval base",
    searchModeWeb: "Web",
    searchModePapers: "Papers",
    searchModeTracking: "Tracking",
    searchModeBenchmarks: "Benchmarks",
    searchModeModels: "Models",
    searchModeDatasets: "Datasets",
    searchModeGitHub: "GitHub",
    searchPlaceholderWeb: "Search public web resources via the web provider",
    searchPlaceholderPapers: "Search paper topics, methods, or benchmarks",
    searchPlaceholderTracking: "Search research-tracking surfaces such as Hugging Face Trending Papers",
    searchPlaceholderBenchmarks: "Search official benchmark platforms such as MLPerf training or inference",
    searchPlaceholderModels: "Search official ONNX Model Zoo models such as bert, resnet, or diffusion",
    searchPlaceholderDatasets: "Search public dataset candidates via OpenML / Hugging Face / Papers With Code / Kaggle",
    searchPlaceholderGitHub: "Search GitHub repositories, code, or dataset repos",
    searchRun: "Search",
    searchOpen: "Open",
    searchUseDataset: "Build manifest",
    searchHealthReady: "Connected",
    searchHealthDown: "Down",
    searchHealthDegraded: "Degraded",
    searchHealthUnknown: "Unknown",
    searchEmpty: "No retrieval results yet.",
    searchLoading: "Searching...",
    searchError: "Search failed.",
    searchDatasetHealth: "Research retrieval base",
    searchWebHealth: "Web provider",
    searchPapersHealth: "Official paper APIs",
    searchTrackingHealth: "Research tracking",
    searchBenchmarksHealth: "Benchmark platforms",
    searchModelsHealth: "ONNX Model Zoo",
    searchGitHubHealth: "GitHub search",
    searchManifestReady: "Manifest created",
    searchNoUrl: "This result has no usable link",
    toastSearchCopied: "Search results refreshed",
    autoSkillsTitle: "Auto-enabled skills this turn",
    autoSkillsKindWorkflow: "Workflow",
    autoSkillsKindSubfield: "Subfield",
    autoSkillsKindGeneral: "General",
    modeChat: "Chat",
    modeResearch: "Agent",
    gitLabel: "Git workspace",
    gitOverview: "Overview",
    gitChanges: "Changes",
    gitHistory: "History",
    gitBranches: "Branches",
    gitGraph: "Graph",
    gitRefresh: "Refresh",
    gitFetch: "Fetch",
    gitPull: "Pull",
    gitPush: "Push",
    gitStageAll: "Stage all",
    gitUnstageAll: "Unstage all",
    gitCommit: "Commit",
    gitCommitPrompt: "Enter commit message",
    gitUnavailable: "Git data is unavailable for the current workspace.",
    gitWorkingTree: "Working tree",
    gitRepository: "Repository",
    gitBranchCurrent: "Current branch",
    gitChangesSummary: "Change summary",
    gitGraphEmpty: "No commit graph is available yet.",
    gitHistoryEmpty: "No commit history is available yet.",
    gitBranchesEmpty: "No branch data is available yet.",
    gitDiffWorking: "Working diff",
    gitDiffStaged: "Staged diff",
    gitNoDiff: "No diff to show.",
    gitCommitAuthor: "Author",
    gitCheckout: "Checkout",
    gitCreateBranch: "New branch",
    gitDeleteBranch: "Delete branch",
    gitBranchPrompt: "Enter branch name",
    gitDeleteBranchConfirm: "Delete branch {branch}?",
    gitStage: "Stage",
    gitUnstage: "Unstage",
    gitDiscard: "Discard",
    gitAheadBehind: "Ahead {ahead} / Behind {behind}",
    gitClean: "Working tree clean",
    gitStaged: "Staged",
    gitModified: "Modified",
    gitUntracked: "Untracked",
    gitConflicted: "Conflicted",
    gitActionFailed: "Git action failed. Please try again.",
    gitNoStatus: "No Git status summary is available.",
  },
};

function normalizeHostMeta(meta) {
  const input = meta || {};
  return {
    mode: input.mode === "desktop" ? "desktop" : "web",
    transport: typeof input.transport === "string" ? input.transport : "http",
    supportsStreaming: input.supportsStreaming !== false,
    supportsFileDialog: input.supportsFileDialog !== false,
    supportsTerminal: input.supportsTerminal !== false,
    supportsTerminalPty: input.supportsTerminalPty === true,
    supportsNativeMenu: input.supportsNativeMenu === true,
    bridgeProtocol: typeof input.bridgeProtocol === "string" ? input.bridgeProtocol : "",
  };
}

function apiUrl(path) {
  const normalized = String(path || "");
  return normalized.startsWith("/") ? normalized : `/${normalized}`;
}

const BRIDGE_COMMANDS = {
  bootstrap: "bootstrap.load",
  settingsUpdate: "settings.update",
  workspacePick: "workspace.pick",
  workspaceOpenFile: "workspace.file.open",
  workspaceSaveFile: "workspace.file.save",
  workspaceUndoFile: "workspace.file.undo",
  workspaceCompleteFile: "workspace.file.complete",
  workspaceReviewFile: "workspace.review.file",
  reviewerFeedbackState: "reviewer_feedback.state",
  reviewerFeedbackAdd: "reviewer_feedback.add",
  reviewerFeedbackResolve: "reviewer_feedback.resolve",
  researchPaperWorkflowRun: "research.paper_workflow.run",
  searchHealth: "search.health",
  searchWeb: "search.web",
  searchPapers: "search.papers",
  searchTracking: "search.tracking",
  searchBenchmarks: "search.benchmarks",
  searchModels: "search.models",
  searchGitHub: "search.github",
  searchGitHubPreview: "search.github_preview",
  searchDatasets: "search.datasets",
  searchDatasetManifest: "search.dataset_manifest",
  browserOpen: "browser.open",
  chatSend: "chat.send",
  chatStream: "chat.stream",
  chatStop: "chat.stop",
  toolApprove: "tool.approval.approve",
  toolDeny: "tool.approval.deny",
  gitState: "git.state",
  gitAction: "git.action",
  extensionsList: "extensions.list",
  runDebugState: "run_debug.state",
  runDebugAction: "run_debug.action",
  terminalsState: "terminals.state",
  terminalsCreate: "terminals.create",
  terminalsInput: "terminals.input",
  terminalsClose: "terminals.close",
  sessionsCreate: "sessions.create",
  sessionsSelect: "sessions.select",
  sessionsRename: "sessions.rename",
  sessionsDelete: "sessions.delete",
};

function createHostClient(meta) {
  const resolved = normalizeHostMeta(meta);
  const desktopBridge = window.__TOKITAI_DESKTOP_BRIDGE__ || null;

  async function bridgeInvoke(command, payload = {}) {
    if (!desktopBridge || typeof desktopBridge.invoke !== "function") {
      throw new Error(`desktop bridge invoke is unavailable for '${command}'`);
    }
    return desktopBridge.invoke(command, payload);
  }

  async function bridgeJsonResponse(command, payload = {}) {
    const result = await bridgeInvoke(command, payload);
    const ok = result?.ok !== false;
    const status = Number(result?.status || (ok ? 200 : 500));
    const data = result?.data;
    const errorText = result?.error || result?.message || "";
    return {
      ok,
      status,
      async json() {
        return data;
      },
      async text() {
        return errorText || JSON.stringify(data ?? {});
      },
    };
  }

  async function request(path, options = {}) {
    if (resolved.mode === "desktop" && resolved.transport === "bridge") {
      const command = String(path || "");
      let payload = {};
      if (typeof options.body === "string" && options.body.length) {
        try {
          payload = JSON.parse(options.body);
        } catch (_error) {
          payload = { rawBody: options.body };
        }
      }
      return bridgeJsonResponse(command, payload);
    }
    return fetch(apiUrl(path), options);
  }

  return {
    meta: resolved,
    request,
    json(path, options = {}) {
      return request(path, options);
    },
    stream(path, options = {}) {
      return request(path, options);
    },
    bootstrap() {
      return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.bootstrap : "/api/bootstrap");
    },
    settings: {
      update(payload) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.settingsUpdate : "/api/settings", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(payload),
        });
      },
    },
    workspace: {
      pick() {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.workspacePick : "/api/workspace/pick", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
        });
      },
      openFile(path) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.workspaceOpenFile : "/api/workspace/file", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ path }),
        });
      },
      saveFile(path, content) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.workspaceSaveFile : "/api/workspace/file/save", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ path, content }),
        });
      },
      undoFile(path, beforeContent) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.workspaceUndoFile : "/api/workspace/file/undo", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ path, before_content: beforeContent }),
        });
      },
      completeFile(payload) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.workspaceCompleteFile : "/api/workspace/file/complete", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(payload),
        });
      },
      reviewFile(path) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.workspaceReviewFile : "/api/review/file", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ path }),
        });
      },
      rawFileUrl(path) {
        const normalized = String(path || "").trim();
        return normalized ? `/api/workspace/file/raw?path=${encodeURIComponent(normalized)}` : "";
      },
    },
    reviewerFeedback: {
      state() {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.reviewerFeedbackState : "/api/reviewer-feedback");
      },
      add(payload) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.reviewerFeedbackAdd : "/api/reviewer-feedback/add", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(payload),
        });
      },
      resolve(index) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.reviewerFeedbackResolve : "/api/reviewer-feedback/resolve", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ index }),
        });
      },
    },
    research: {
      paperWorkflow(payload) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.researchPaperWorkflowRun : "/api/research/paper-workflow", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(payload),
        });
      },
    },
    search: {
      health() {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.searchHealth : "/api/search/health");
      },
      web(query, limit = 8) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.searchWeb : "/api/search/web", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ query, limit }),
        });
      },
      papers(query, source = "auto", limit = 8) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.searchPapers : "/api/search/papers", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ query, source, limit }),
        });
      },
      tracking(query, source = "auto", limit = 8) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.searchTracking : "/api/search/tracking", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ query, source, limit }),
        });
      },
      benchmarks(query, source = "mlperf", limit = 8) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.searchBenchmarks : "/api/search/benchmarks", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ query, source, limit }),
        });
      },
      models(query, limit = 8) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.searchModels : "/api/search/models", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ query, limit }),
        });
      },
      github(query, mode = "repositories", limit = 8) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.searchGitHub : "/api/search/github", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ query, mode, limit }),
        });
      },
      githubPreview(repoFullName, branch = null, path = null, commitSha = null, compareBaseSha = null, compareHeadSha = null, historyScopeMode = null) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.searchGitHubPreview : "/api/search/github/preview", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            repo_full_name: repoFullName,
            branch,
            path,
            commit_sha: commitSha,
            compare_base_sha: compareBaseSha,
            compare_head_sha: compareHeadSha,
            history_scope_mode: historyScopeMode,
          }),
        });
      },
      datasets(query, limit = 8) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.searchDatasets : "/api/search/datasets", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ query, limit }),
        });
      },
      datasetManifest(datasetUrl, title = null) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.searchDatasetManifest : "/api/search/dataset-manifest", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ dataset_url: datasetUrl, title }),
        });
      },
      browserOpen(url) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.browserOpen : "/api/browser/open", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ url }),
        });
      },
    },
    chat: {
      send(content, language = currentLanguage) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.chatSend : "/api/send", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ content, language }),
        });
      },
      stream(payload) {
        if (resolved.mode === "desktop" && resolved.transport === "bridge") {
          if (!desktopBridge || typeof desktopBridge.openStream !== "function") {
            throw new Error("desktop stream bridge is unavailable");
          }
          return desktopBridge.openStream(BRIDGE_COMMANDS.chatStream, payload);
        }
        return fetch(apiUrl("/api/send-stream"), {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(payload),
        });
      },
      stop(sessionId) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.chatStop : "/api/send-stop", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ session_id: sessionId }),
        });
      },
    },
    toolApproval: {
      approve(sessionId, callId) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.toolApprove : "/api/tool/approve", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ session_id: sessionId, call_id: callId }),
        });
      },
      deny(sessionId, callId) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.toolDeny : "/api/tool/deny", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ session_id: sessionId, call_id: callId }),
        });
      },
    },
    git: {
      state(options = {}) {
        const payload = {
          diff: options.diff === true,
          graph: options.graph === true,
        };
        if (resolved.transport === "bridge") {
          return request(BRIDGE_COMMANDS.gitState, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(payload),
          });
        }
        const params = new URLSearchParams();
        if (payload.diff) params.set("diff", "true");
        if (payload.graph) params.set("graph", "true");
        const suffix = params.toString();
        return request(`/api/git${suffix ? `?${suffix}` : ""}`);
      },
      action(action, extra = {}) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.gitAction : "/api/git/action", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ action, ...extra }),
        });
      },
    },
    extensions: {
      list() {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.extensionsList : "/api/extensions");
      },
    },
    runDebug: {
      state() {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.runDebugState : "/api/run-debug");
      },
      action(action, configId = null) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.runDebugAction : "/api/run-debug/action", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ action, config_id: configId }),
        });
      },
    },
    terminals: {
      state() {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.terminalsState : "/api/terminals");
      },
      create() {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.terminalsCreate : "/api/terminals/create", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
        });
      },
      input(terminalId, input) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.terminalsInput : "/api/terminals/input", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ terminal_id: terminalId, input }),
        });
      },
      close(terminalId) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.terminalsClose : "/api/terminals/close", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ terminal_id: terminalId }),
        });
      },
    },
    sessions: {
      create() {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.sessionsCreate : "/api/sessions", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
        });
      },
      select(sessionId) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.sessionsSelect : "/api/sessions/select", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ session_id: sessionId }),
        });
      },
      rename(sessionId, title) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.sessionsRename : "/api/sessions/rename", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ session_id: sessionId, title }),
        });
      },
      delete(sessionId) {
        return request(resolved.transport === "bridge" ? BRIDGE_COMMANDS.sessionsDelete : "/api/sessions/delete", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ session_id: sessionId }),
        });
      },
    },
  };
}

const hostClient = createHostClient(window.__TOKITAI_HOST__);
let currentHostMeta = { ...hostClient.meta };

const RESEARCH_PHASES = [
  { key: "phaseLiterature", noteKey: "noteLiterature", aliases: ["literature review"] },
  { key: "phaseHypothesis", noteKey: "noteHypothesis", aliases: ["generating hypothesis", "hypothesis generation"] },
  { key: "phaseExperiment", noteKey: "noteExperiment", aliases: ["designing experiment", "experiment design"] },
  { key: "phaseExecution", noteKey: "noteExecution", aliases: ["executing experiment", "execution"] },
  { key: "phaseValidation", noteKey: "noteValidation", aliases: ["validating results", "validation"] },
  { key: "phasePaper", noteKey: "notePaper", aliases: ["writing paper", "paper writing"] },
  { key: "phaseReview", noteKey: "noteReview", aliases: ["review"] },
];

let currentLanguage = "zh";
let currentEffort = "medium";
let effortPersistTimer = null;
let effortPersistRequestId = 0;
let agentPreludeHideTimer = null;
let bootstrapData = null;
let activeSettingsPanel = null;
let activeSettingsTab = "model";
let activeSessionMenuId = null;
let activeSessionMenuAnchor = null;
let toastTimer = null;
let pendingFiles = [];
let isSending = false;
const sessionRunState = new Map();
let pendingUserBubble = null;
let pendingAssistantBubble = null;
let activityTimer = null;
let activityStartedAt = null;
let activityLabel = "";
let activityPillNode = null;
let activityPillLabelNode = null;
let activityPillTimeNode = null;
let activeReviewFilePath = null;
let reviewDetailCache = new Map();
const undoSnapshotCache = new Map();
const diffReviewState = new Map();
let undoSnapshotSequence = 0;
let currentWorkspaceMode = "chat";
let currentMainView = "chat";
let currentGitView = "overview";
let gitDataLoadState = { diff: false, graph: false };
let gitLoadPromise = null;
let activeActivityPanel = null;
let preferredLeftActivityPanel = "nav";
let extensionCatalog = [];
let searchMode = "web";
let searchState = {
  loading: false,
  health: null,
  results: null,
  error: "",
  manifests: {},
  activeManifestUrl: "",
  githubPreview: null,
  githubPreviewLoading: false,
  githubPreviewError: "",
  githubPreviewSourceIndex: -1,
  githubPreviewHistory: [],
  githubPreviewHistoryIndex: -1,
  lastQuery: "",
};
let browserState = {
  currentUrl: "",
  currentTitle: "",
  currentViewUrl: "",
  history: [],
  historyIndex: -1,
  loading: false,
  blankReloadAttempts: 0,
  renderRequestId: 0,
};
let preferredDockRightSidebarPanelId = "tree";
let rightSidebarCollapsed = true;

function syncBrowserStateFromFrame(options = {}) {
  if (!browserFrame) return;
  try {
    const frameDocument = browserFrame.contentDocument;
    if (!frameDocument) return;
    const nextUrl = cleanDisplayText(browserState.currentUrl || "");
    const nextTitle = cleanDisplayText(frameDocument.title || "");
    const pushHistory = options.pushHistory !== false;
    if (nextUrl) {
      browserState.currentUrl = nextUrl;
      browserState.currentTitle = nextTitle;
      if (pushHistory && browserState.history[browserState.historyIndex] !== nextUrl) {
        browserState.history = browserState.history.slice(0, browserState.historyIndex + 1);
        browserState.history.push(nextUrl);
        browserState.historyIndex = browserState.history.length - 1;
      }
      if (browserToolbarAddress) {
        browserToolbarAddress.textContent = nextTitle ? `${nextTitle} - ${nextUrl}` : nextUrl;
      }
    }
  } catch (error) {
    console.warn("failed to sync browser frame state", error);
  }
}

function closeInAppBrowser() {
  browserState.currentUrl = "";
  browserState.currentTitle = "";
  browserState.currentViewUrl = "";
  browserState.history = [];
  browserState.historyIndex = -1;
  browserState.blankReloadAttempts = 0;
  browserState.renderRequestId += 1;
  if (browserFrame) {
    browserFrame.removeAttribute("srcdoc");
    delete browserFrame.dataset.viewUrl;
    browserFrame.src = "about:blank";
  }
  if (browserToolbarAddress) {
    browserToolbarAddress.textContent = "No page loaded";
  }
  setMainView("chat");
  applyDockLayout();
  syncLayoutCornerControls();
}

async function loadInAppBrowserDocument(data, fallbackHref) {
  if (!browserFrame) return;
  browserState.blankReloadAttempts = 0;
  const viewUrl = cleanDisplayText(
    data?.view_url || browserState.currentViewUrl || `/api/browser/view?url=${encodeURIComponent(fallbackHref || browserState.currentUrl || "")}`,
  );
  browserState.currentViewUrl = viewUrl;
  const requestId = ++browserState.renderRequestId;
  const response = await fetch(viewUrl, { cache: "no-store" });
  const html = await response.text();
  if (requestId !== browserState.renderRequestId) return;
  const frameDocument = browserFrame.contentDocument || browserFrame.contentWindow?.document;
  if (!frameDocument) {
    browserFrame.src = viewUrl;
    return;
  }
  browserFrame.dataset.viewUrl = viewUrl;
  frameDocument.open();
  frameDocument.write(html);
  frameDocument.close();
  window.setTimeout(() => {
    if (requestId !== browserState.renderRequestId) return;
    syncBrowserStateFromFrame({ pushHistory: false });
  }, 30);
}

function scheduleBrowserBlankCheck(expectedUrl) {
  if (!browserFrame) return;
  const attemptToken = ++browserState.blankReloadAttempts;
  window.setTimeout(async () => {
    if (!browserFrame || browserState.currentUrl !== expectedUrl) return;
    if (attemptToken !== browserState.blankReloadAttempts) return;
    let shouldRetry = false;
    try {
      const body = browserFrame.contentDocument?.body || null;
      const textLen = (body?.innerText || "").trim().length;
      const htmlLen = (body?.innerHTML || "").trim().length;
      const href = browserFrame.contentWindow?.location?.href || "";
      shouldRetry = textLen === 0 && htmlLen === 0 && (!href || href === "about:blank");
    } catch (_error) {
      shouldRetry = false;
    }
    if (!shouldRetry) return;
    try {
      await loadInAppBrowserDocument({}, expectedUrl);
    } catch (error) {
      console.error(error);
    }
  }, 900);
}
let runDebugState = null;
let terminalState = { sessions: [], active_id: null };
const reviewerFeedbackDrafts = new Map();
const reviewerFeedbackPendingSessions = new Set();
const paperWorkflowPendingSessions = new Set();
const paperWorkflowAutoTriggeredSessions = new Set();
const paperWorkflowPromptDismissedSessions = new Set();
let terminalDrawerDismissed = false;
let terminalPollTimer = null;
let researchFloatingDismissed = false;
let researchFloatingDrag = null;
let researchFloatingReopenDrag = null;
let researchFloatingReopenSuppressClick = false;
let researchFloatingBoardPosition = null;
let preservedMessageScrollState = null;
let workspaceTreeData = [];
let activeWorkspaceFilePath = null;
let expandedWorkspaceDirs = new Set();
let isWorkspaceCodeOpen = false;
let workspaceFileClickTimer = null;
let workspaceCodeRenderMode = "source";
let currentWorkspaceFile = null;
let workspaceEditorDirty = false;
let workspaceOpenTabs = [];
let workspaceDraftCache = new Map();
let workspaceRenderModeByPath = new Map();
let workspaceEditorComposing = false;
let workspaceCompletionTimer = null;
let workspaceCompletionRequestId = 0;
let workspaceCompletionState = {
  items: [],
  visible: false,
  activeIndex: 0,
  tokenPrefix: "",
  tokenStart: 0,
  tokenEnd: 0,
  lastQuery: "",
};
let workspaceCodeResizeObserver = null;
let workspaceMonacoLoaderPromise = null;
let workspaceMonacoEditor = null;
let workspaceMonacoModel = null;
let workspaceMonacoLanguageId = "";
let workspaceMonacoRenderToken = 0;
let workspaceMonacoRegisteredLanguages = new Set();
let workspaceMonacoCompletionProviders = new Set();
let workspaceMonacoThemeReady = false;
let workspaceMonacoModelCache = new Map();
let workspaceMonacoViewStateCache = new Map();
let paperWorkspaceState = {
  sectionId: "",
  path: "",
  label: "",
  file: null,
  loading: false,
  error: "",
};
function resetPaperWorkspaceState() {
  paperWorkspaceState.sectionId = "";
  paperWorkspaceState.path = "";
  paperWorkspaceState.label = "";
  paperWorkspaceState.file = null;
  paperWorkspaceState.loading = false;
  paperWorkspaceState.error = "";
}
let workspaceMonacoDefinitionProviders = new Set();
let workspaceMonacoSymbolProviders = new Set();
let workspaceMonacoHoverProviders = new Set();
let workspaceFileTextCache = new Map();
let workspaceSymbolIndexCache = new Map();
let workspaceDiagnosticsTimer = null;
let workspacePendingReveal = null;
let workspaceReferenceMatches = [];
let preserveWorkspaceSlotWhenCodeClosed = true;

function currentGitFetchOptions(view = currentGitView) {
  return {
    diff: view === "changes",
    graph: view === "graph",
  };
}

function setGitLoadState(options = {}) {
  gitDataLoadState = {
    diff: options.diff === true,
    graph: options.graph === true,
  };
}

function scheduleTerminalPoll(delay = 3000) {
  window.clearTimeout(terminalPollTimer);
  terminalPollTimer = null;
  if (document.hidden) return;
  if (!terminalDrawer || terminalDrawer.hidden) return;
  terminalPollTimer = window.setTimeout(async () => {
    try {
      await loadTerminalState();
    } catch (_error) {
      // ignore polling failures
    } finally {
      scheduleTerminalPoll(3000);
    }
  }, delay);
}

let katexEnsurePromise = null;

async function ensureKatexReady() {
  if (window.katex?.renderToString) return true;
  if (katexEnsurePromise) return katexEnsurePromise;
  katexEnsurePromise = new Promise((resolve) => {
    const script = document.createElement("script");
    script.src = `./vendor/katex/katex.min.js?v=20260619m`;
    script.async = true;
    script.onload = () => resolve(Boolean(window.katex?.renderToString));
    script.onerror = () => resolve(false);
    document.head.appendChild(script);
  }).finally(() => {
    katexEnsurePromise = null;
  });
  return katexEnsurePromise;
}

async function refreshLatexRendering() {
  const ready = await ensureKatexReady();
  if (!ready) return false;
  if (bootstrapData) {
    renderFromState();
    if (currentWorkspaceFile) {
      renderWorkspaceFile(currentWorkspaceFile);
    }
  }
  if (pendingAssistantBubble && activeAssistantTurn) {
    syncPendingAssistantText();
  }
  return true;
}
let currentWorkspaceRoot = "";
let activeDockDrag = null;
let activeResizerDrag = null;
let gripHoldTimer = null;
let suppressNextGripClick = false;
let lastGripPointerDownAt = 0;
let currentStreamingSessionId = null;
let activeStreamGeneration = 0;
let pendingPermissionRequest = null;
let liveToolEvents = [];
let liveEditedFiles = [];
let liveProcessEvents = [];
let pinnedEditedFiles = [];
let activeAssistantTurn = null;
let pendingAssistantRuntimeNode = null;
let pendingAssistantRuntimeStatusesNode = null;
let pendingAssistantStoryNode = null;
let pendingAssistantOperationsNode = null;
let pendingAssistantThinkingHost = null;
let pendingAssistantTextNode = null;
let pendingAssistantThinkingNode = null;
let pendingAssistantStableNode = null;
let pendingAssistantTailNode = null;
let pendingAssistantStatusTextNode = null;
let pendingAssistantStatusTimeNode = null;
let pendingAssistantRenderedRuntimeText = null;
let pendingAssistantRenderedStableText = null;
let pendingAssistantRenderedTailText = null;
let pendingAssistantRenderedOperationsHtml = null;
let pendingAssistantThinkingDirty = false;
let pendingAssistantStoryDirty = false;
let pendingAssistantOperationsDirty = false;
let preservedThinking = [];
let pendingAssistantTextFrame = null;
let pendingAssistantStatusFrame = null;
let pendingAssistantBubbleFrame = null;
let messageStreamFollowFrame = null;
let messageStreamFollowTarget = 0;
let pendingBootstrapRefreshPromise = null;
let suppressVisibleStreamBootstrap = false;
let lastVisibleCompletionSignature = "";
let autoOpenActivityPanel = false;
const MAX_LIVE_PROCESS_EVENTS = 4;
let researchDetailOpen = false;

try {
  currentLanguage = localStorage.getItem("tokitai-language") || "zh";
} catch (_error) {
  currentLanguage = "zh";
}

function restoreLayoutPreferences() {
  try {
    const savedEffort = String(localStorage.getItem("tokitai-effort") || "").toLowerCase();
    if (["low", "medium", "high", "max"].includes(savedEffort)) {
      currentEffort = savedEffort;
    }

    const savedMode = String(localStorage.getItem("tokitai-workspace-mode") || "").toLowerCase();
    if (["chat", "research"].includes(savedMode)) {
      currentWorkspaceMode = savedMode;
    }

    autoOpenActivityPanel = localStorage.getItem("tokitai-auto-open-activity-panel") === "true";
  } catch (_error) {
    // Ignore storage failures.
  }
}

restoreLayoutPreferences();

const sessionList = document.querySelector(".session-list");
const currentSessionList = document.getElementById("current-session-list");
const branchList = document.querySelector(".branch-list");
const messageStream = document.querySelector(".message-stream");
const reviewStrip = document.getElementById("review-strip");
const agentPreludeBackground = document.getElementById("agent-prelude-background");
const agentPreludeUnicorn = document.getElementById("agent-prelude-unicorn");
const agentPreludeSplineFrame = document.getElementById("agent-prelude-spline-frame");
const appShell = document.querySelector(".app-shell");
const messageInput = document.getElementById("message-input");
const attachButton = document.getElementById("attach-button");
const fileInput = document.getElementById("file-input");
const composerAttachments = document.getElementById("composer-attachments");
const langToggle = document.getElementById("lang-toggle");
const autoOpenActivityPanelToggle = document.getElementById("auto-open-activity-panel");
const toast = document.getElementById("toast");
const sidebarWorkspaceTitle = document.getElementById("sidebar-workspace-title");
const workspaceRootLabel = document.getElementById("workspace-root-label");
const workspaceTitle = document.getElementById("workspace-title");
const riskPill = document.getElementById("risk-pill");
const primaryModel = document.getElementById("primary-model");
const primaryApiUrl = document.getElementById("primary-api-url");
const competitionMode = document.getElementById("competition-mode");
const privacyMode = document.getElementById("privacy-mode");
const deepThinkToggle = document.getElementById("deep-think");
const autoApproveTools = document.getElementById("auto-approve-tools");
const riskBoundary = document.getElementById("risk-boundary");
const maxToolCalls = document.getElementById("max-tool-calls");
const burstLimit = document.getElementById("burst-limit");
const runtimeWorkspaceRoot = document.getElementById("runtime-workspace-root");
const runtimeApiKey = document.getElementById("runtime-api-key");
const runtimeToolchainInputs = document.querySelectorAll("[data-toolchain-key]");
const providerList = document.getElementById("provider-list");
const workspacePickerToggle = document.getElementById("workspace-picker-toggle");
const activityRail = document.getElementById("activity-rail");
const activityFlyout = document.getElementById("activity-flyout");
const activityFlyoutResizer = document.getElementById("activity-flyout-resizer");
const activityRailButtons = document.querySelectorAll("[data-activity-panel]");
const activityPanels = document.querySelectorAll("[data-activity-panel-id]");
const extensionSearchInput = document.getElementById("extension-search-input");
const extensionList = document.getElementById("extension-list");
const searchModeSwitch = document.getElementById("search-mode-switch");
const searchQueryInput = document.getElementById("search-query-input");
const searchRunButton = document.getElementById("search-run-button");
const searchResults = document.getElementById("search-results");
const searchHealthStrip = document.getElementById("search-health-strip");
const searchPreviewPanel = document.getElementById("search-preview-panel");
const runDebugList = document.getElementById("run-debug-list");
const runDebugSession = document.getElementById("run-debug-session");
const terminalRailButton = document.getElementById("terminal-rail-button");
const terminalDrawer = document.getElementById("terminal-drawer");
const terminalScreen = document.getElementById("terminal-screen");
const terminalTabList = document.getElementById("terminal-tab-list");
const terminalOutput = document.getElementById("terminal-output");
const terminalInput = document.getElementById("terminal-input");
const terminalHideButton = document.getElementById("terminal-hide-button");
const terminalNewInline = document.getElementById("terminal-new-inline");
const activityStrip = document.getElementById("activity-strip");
const agentRuntimeStrip = document.getElementById("agent-runtime-strip");
const agentProcessStrip = document.getElementById("agent-process-strip");
const permissionStrip = document.getElementById("permission-strip");
const contextUsage = document.getElementById("context-usage");
const contextUsageRing = document.getElementById("context-usage-ring");
const contextUsageLabel = document.getElementById("context-usage-label");
const settingsPanels = document.querySelectorAll(".settings-popover");
const newSessionButton = document.getElementById("new-session-button");
const settingsToggle = document.getElementById("settings-toggle");
const settingsPanel = document.getElementById("settings-panel");
const settingsClose = document.getElementById("settings-close");
const settingsTabs = document.querySelectorAll("[data-settings-tab]");
const settingsTabPanels = document.querySelectorAll("[data-settings-tab-panel]");
const settingsSaveButton = document.getElementById("settings-save");
const effortDisclosure = document.getElementById("effort-disclosure");
const effortSlider = document.getElementById("effort-slider");
const effortTriggerValue = document.getElementById("effort-trigger-value");
const effortPanelTitle = document.getElementById("effort-panel-title");
const effortPanelMeta = document.getElementById("effort-panel-meta");
const effortButtons = document.querySelectorAll("[data-effort]");
const sessionMenu = document.getElementById("session-menu");
const sessionMenuRename = document.getElementById("session-menu-rename");
const sessionMenuDelete = document.getElementById("session-menu-delete");
const researchPanel = document.getElementById("research-panel");
const researchSection = document.querySelector(".sidebar-section-research");
const researchFloatingBoard = document.getElementById("research-floating-board");
const researchFloatingHead = document.getElementById("research-floating-head");
const researchFloatingBody = document.getElementById("research-floating-body");
const researchFloatingHide = document.getElementById("research-floating-hide");
const researchFloatingReopen = document.getElementById("research-floating-reopen");
const researchDetailPanel = document.getElementById("research-detail-panel");
const segmentedControls = document.querySelectorAll("[data-segmented]");
const modeButtons = document.querySelectorAll("[data-mode]");
const workspaceTree = document.getElementById("workspace-tree");
const workspaceFilesSubtitle = document.getElementById("workspace-files-subtitle");
const workspaceCodeContent = document.getElementById("workspace-code-content");
const workspaceCodePath = document.getElementById("workspace-code-path");
const workspaceCodeMeta = document.getElementById("workspace-code-meta");
const workspaceCodeSaveButton = document.getElementById("workspace-code-save");
const workspaceCodeRenderToggle = document.getElementById("workspace-code-render-toggle");
const workspaceCodeTabs = document.getElementById("workspace-code-tabs");
const workspaceCodeBreadcrumbs = document.getElementById("workspace-code-breadcrumbs");
const workspaceCodeSearchButton = document.getElementById("workspace-code-search");
const workspaceCodeRunButton = document.getElementById("workspace-code-run");
const workspaceCodeReplaceButton = document.getElementById("workspace-code-replace");
const workspaceCodeLineButton = document.getElementById("workspace-code-line");
const workspaceCodeSymbolsButton = document.getElementById("workspace-code-symbols");
const workspaceCodeReferencesButton = document.getElementById("workspace-code-references");
const workspaceCodeRenameButton = document.getElementById("workspace-code-rename");
const workspaceLauncher = document.getElementById("workspace-launcher");
const dockWorkspace = document.getElementById("dock-workspace");
const panelMenu = document.getElementById("panel-menu");
const panelGrips = document.querySelectorAll("[data-panel-grip]");
const panelResizers = document.querySelectorAll("[data-resizer-after]");
const activityCollapseButtons = document.querySelectorAll("[data-collapse-activity]");
const leftSidebarToggleButton = document.getElementById("left-sidebar-toggle");
const rightSidebarToggleButton = document.getElementById("right-sidebar-toggle");
const gitNav = document.getElementById("git-nav");
const gitWorkspace = document.getElementById("git-workspace");
const browserSplitResizer = document.getElementById("browser-split-resizer");
const browserWorkspace = document.getElementById("browser-workspace");
const browserFrame = document.getElementById("browser-frame");
const browserBackButton = document.getElementById("browser-back-button");
const browserRefreshButton = document.getElementById("browser-refresh-button");
const browserExternalButton = document.getElementById("browser-external-button");
const browserCloseButton = document.getElementById("browser-close-button");
const browserToolbarAddress = document.getElementById("browser-toolbar-address");
const gitStatusBanner = document.getElementById("git-status-banner");
const gitOverviewView = document.getElementById("git-view-overview");
const gitChangesView = document.getElementById("git-view-changes");
const gitHistoryView = document.getElementById("git-view-history");
const gitBranchesView = document.getElementById("git-view-branches");
const gitGraphView = document.getElementById("git-view-graph");
const gitRefreshButton = document.getElementById("git-refresh-button");
const gitFetchButton = document.getElementById("git-fetch-button");
const gitPullButton = document.getElementById("git-pull-button");
const gitPushButton = document.getElementById("git-push-button");
const gitStageAllButton = document.getElementById("git-stage-all-button");
const gitUnstageAllButton = document.getElementById("git-unstage-all-button");
const gitCommitButton = document.getElementById("git-commit-button");
const composerStop = document.getElementById("composer-stop");

const DOCK_LAYOUT_KEY = "tokitai-dock-layout-v1";
const RESEARCH_STARTED_KEY = "tokitai-research-started-v1";
const SANDBOX_NOTICE_KEY = "tokitai-sandbox-notice-v1";
const PANEL_IDS = ["sidebar", "chat", "research", "code", "tree"];
const LEFT_ACTIVITY_ORDER = ["nav", "extensions", "search", "git", "run"];
const RIGHT_DOCK_PANEL_IDS = ["tree", "code", "research"];
const DEFAULT_DOCK_LAYOUT = {
  order: ["sidebar", "chat", "research", "code", "tree"],
  hidden: { sidebar: true, chat: false, research: true, tree: true, code: true },
  widths: { sidebar: 280, chat: 1, research: 380, tree: 320, code: 860, flyout: 304, browser: 520 },
};
const MIN_ACTIVITY_FLYOUT_WIDTH = 248;
const MAX_ACTIVITY_FLYOUT_WIDTH = 820;
const SEARCH_GITHUB_FLYOUT_BREAKPOINT = 1120;
const SEARCH_GITHUB_FLYOUT_MIN_WIDTH = 760;
const SEARCH_GITHUB_FLYOUT_MAX_WIDTH = 980;

function t(key) {
  return translations[currentLanguage]?.[key] || translations.en[key] || key;
}

function template(key, values = {}) {
  return String(t(key)).replace(/\{(\w+)\}/g, (_match, name) => String(values[name] ?? ""));
}

function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

function setRootCssVar(name, value) {
  if (typeof document === "undefined") return;
  document.documentElement.style.setProperty(name, value);
}

function isSearchGitHubPreviewActive() {
  if (!(activeActivityPanel === "search" && searchMode === "github")) {
    return false;
  }
  if (searchState?.githubPreviewLoading || searchState?.githubPreviewError) {
    return true;
  }
  return Boolean(searchState?.githubPreview && typeof searchState.githubPreview === "object");
}

function resolveSearchGitHubFlyoutWidth(baseWidth) {
  const viewportWidth = Number(window?.innerWidth || 0);
  if (viewportWidth < SEARCH_GITHUB_FLYOUT_BREAKPOINT) {
    return baseWidth;
  }
  const availableWidth = Math.max(0, viewportWidth - 56);
  const preservedMainWidth = viewportWidth >= 1320 ? 420 : 360;
  const recommendedWidth = clamp(
    availableWidth - preservedMainWidth,
    SEARCH_GITHUB_FLYOUT_MIN_WIDTH,
    SEARCH_GITHUB_FLYOUT_MAX_WIDTH,
  );
  return Math.max(baseWidth, recommendedWidth);
}

function syncShellLayoutVars() {
  const flyoutWidth = clamp(
    Number(dockLayout?.widths?.flyout || DEFAULT_DOCK_LAYOUT.widths.flyout),
    MIN_ACTIVITY_FLYOUT_WIDTH,
    MAX_ACTIVITY_FLYOUT_WIDTH,
  );
  const effectiveFlyoutWidth = isSearchGitHubPreviewActive()
    ? resolveSearchGitHubFlyoutWidth(flyoutWidth)
    : flyoutWidth;
  const browserWidth = clamp(Number(dockLayout?.widths?.browser || DEFAULT_DOCK_LAYOUT.widths.browser), 360, 760);
  if (dockLayout?.widths) {
    dockLayout.widths.flyout = flyoutWidth;
    dockLayout.widths.browser = browserWidth;
  }
  setRootCssVar("--activity-flyout-width", `${flyoutWidth}px`);
  setRootCssVar("--activity-flyout-effective-width", `${effectiveFlyoutWidth}px`);
  setRootCssVar("--browser-panel-width", `${browserWidth}px`);
}

function readDockLayout() {
  try {
    const raw = localStorage.getItem(DOCK_LAYOUT_KEY);
    if (!raw) return structuredClone(DEFAULT_DOCK_LAYOUT);
    const parsed = JSON.parse(raw);
    const widths = { ...DEFAULT_DOCK_LAYOUT.widths, ...(parsed.widths || {}) };
    widths.sidebar = clamp(Number(widths.sidebar || DEFAULT_DOCK_LAYOUT.widths.sidebar), 220, 420);
    widths.research = clamp(Number(widths.research || DEFAULT_DOCK_LAYOUT.widths.research), 320, 460);
    widths.tree = clamp(Number(widths.tree || DEFAULT_DOCK_LAYOUT.widths.tree), 300, 420);
    widths.code = clamp(Number(widths.code || DEFAULT_DOCK_LAYOUT.widths.code), 560, 1400);
    widths.flyout = clamp(
      Number(widths.flyout || DEFAULT_DOCK_LAYOUT.widths.flyout),
      MIN_ACTIVITY_FLYOUT_WIDTH,
      MAX_ACTIVITY_FLYOUT_WIDTH,
    );
    widths.browser = clamp(Number(widths.browser || DEFAULT_DOCK_LAYOUT.widths.browser), 360, 760);
    return {
      order: Array.isArray(parsed.order)
        ? PANEL_IDS.filter((id) => parsed.order.includes(id)).concat(PANEL_IDS.filter((id) => !parsed.order.includes(id)))
        : [...DEFAULT_DOCK_LAYOUT.order],
      hidden: { ...DEFAULT_DOCK_LAYOUT.hidden, ...(parsed.hidden || {}) },
      widths,
    };
  } catch (_error) {
    return structuredClone(DEFAULT_DOCK_LAYOUT);
  }
}

let dockLayout = readDockLayout();
// Force right sidebar panels collapsed on startup to keep clean UI
dockLayout.hidden.tree = true;
dockLayout.hidden.code = true;
dockLayout.hidden.research = true;
rightSidebarCollapsed = true;
syncShellLayoutVars();
let pendingResearchStart = false;

function readResearchStartedSessions() {
  try {
    const raw = localStorage.getItem(RESEARCH_STARTED_KEY);
    const parsed = raw ? JSON.parse(raw) : [];
    return new Set(Array.isArray(parsed) ? parsed.filter((value) => typeof value === "string" && value.trim()) : []);
  } catch (_error) {
    return new Set();
  }
}

let researchStartedSessions = readResearchStartedSessions();

if (researchFloatingBoard && researchFloatingBoard.parentElement !== document.body) {
  document.body.appendChild(researchFloatingBoard);
}
if (researchFloatingReopen && researchFloatingReopen.parentElement !== document.body) {
  document.body.appendChild(researchFloatingReopen);
}

function saveResearchStartedSessions() {
  try {
    localStorage.setItem(RESEARCH_STARTED_KEY, JSON.stringify(Array.from(researchStartedSessions)));
  } catch (_error) {
    // Ignore storage failures.
  }
}

function markResearchStartedForSession(sessionId) {
  const id = String(sessionId || "").trim();
  if (!id || researchStartedSessions.has(id)) return;
  researchStartedSessions.add(id);
  saveResearchStartedSessions();
}

function unmarkResearchStartedForSession(sessionId) {
  const id = String(sessionId || "").trim();
  if (!id || !researchStartedSessions.has(id)) return;
  researchStartedSessions.delete(id);
  saveResearchStartedSessions();
}

function hasResearchStartedForCurrentSession() {
  const id = String(bootstrapData?.current_session_id || "").trim();
  return Boolean(id) && researchStartedSessions.has(id);
}

function resultBundleFieldValue(resultBundle, name) {
  const key = String(name || "").trim();
  if (!key) return "";
  const fields = Array.isArray(resultBundle?.summary_fields) ? resultBundle.summary_fields : [];
  const match = fields.find((field) => normalizeText(field?.name || "") === normalizeText(key));
  return cleanDisplayText(match?.value || "", "");
}

function isPaperWorkflowReadyForAutoRun(research) {
  const backendPaperReady = research?.paper_ready || null;
  if (!research || research.paper_workflow) return false;
  if (backendPaperReady?.workflow_present) return false;
  if (backendPaperReady?.ready) return false;
  const backendReason = normalizeText(backendPaperReady?.reason || "");
  if (backendPaperReady && backendReason && !backendReason.includes("no server-side paper workflow exists yet")) {
    return false;
  }
  if (!research.active) return false;
  if (!hasResearchStartedForCurrentSession()) return false;
  if (research.waiting_approval) return false;
  const overallState = String(resolveResearchOverallState(research) || "").trim().toLowerCase();
  if (!["complete", "resumable"].includes(overallState)) return false;
  const verifierStatus = String(research?.runtime?.verifier?.status || "").trim().toLowerCase();
  if (verifierStatus && !["pass", "complete"].includes(verifierStatus)) return false;
  return Boolean(resultBundleFieldValue(research?.result_bundle || null, "run_id"));
}

function shouldShowPaperWorkflowPrompt(research) {
  const sessionId = String(bootstrapData?.current_session_id || "").trim();
  if (!sessionId) return false;
  if (paperWorkflowPromptDismissedSessions.has(sessionId)) return false;
  return isPaperWorkflowReadyForAutoRun(research);
}

async function ensureSessionReady() {
  let sessionId = String(bootstrapData?.current_session_id || "").trim();
  if (sessionId) return sessionId;
  try {
    await loadBootstrap();
  } catch (_error) {
    // ignore and try session creation below
  }
  sessionId = String(bootstrapData?.current_session_id || "").trim();
  if (sessionId) return sessionId;
  try {
    const response = await hostClient.sessions.create();
    if (!response.ok) {
      return "";
    }
    await loadBootstrap();
  } catch (_error) {
    return "";
  }
  sessionId = String(bootstrapData?.current_session_id || "").trim();
  return sessionId;
}

function saveDockLayout() {
  try {
    localStorage.setItem(DOCK_LAYOUT_KEY, JSON.stringify(dockLayout));
  } catch (_error) {
    // Ignore storage failures.
  }
}

function visiblePanelIds() {
  return dockLayout.order.filter((id) => !dockLayout.hidden[id]);
}

function renderedPanelIds() {
  return visiblePanelIds();
}

function isMarkdownWorkspaceFile(file) {
  const path = String(file?.path || file?.name || "").toLowerCase();
  const language = String(file?.language || "").toLowerCase();
  return path.endsWith(".md") || path.endsWith(".markdown") || language === "markdown";
}

function workspaceFilePreviewKind(file) {
  return String(file?.preview_kind || "").toLowerCase() || "text";
}

function workspaceRawFileUrl(file) {
  const path = String(file?.path || "");
  return hostClient.workspace.rawFileUrl(path);
}

function applyHostMeta(meta) {
  currentHostMeta = normalizeHostMeta({
    ...currentHostMeta,
    ...(meta || {}),
    supportsStreaming: meta?.supports_streaming ?? meta?.supportsStreaming ?? currentHostMeta.supportsStreaming,
    supportsFileDialog: meta?.supports_file_dialog ?? meta?.supportsFileDialog ?? currentHostMeta.supportsFileDialog,
    supportsTerminal: meta?.supports_terminal ?? meta?.supportsTerminal ?? currentHostMeta.supportsTerminal,
    supportsTerminalPty: meta?.supports_terminal_pty ?? meta?.supportsTerminalPty ?? currentHostMeta.supportsTerminalPty,
    supportsNativeMenu: meta?.supports_native_menu ?? meta?.supportsNativeMenu ?? currentHostMeta.supportsNativeMenu,
    bridgeProtocol: meta?.bridge_protocol ?? meta?.bridgeProtocol ?? currentHostMeta.bridgeProtocol,
  });
}

function workspaceFileMetaText(file) {
  if (!file) return "";
  const previewKind = workspaceFilePreviewKind(file);
  const mimeType = String(file?.mime_type || "").toLowerCase();
  const labelMap = {
    image: currentLanguage === "zh" ? "图片预览" : "Image preview",
    pdf: currentLanguage === "zh" ? "PDF 文档" : "PDF document",
    audio: currentLanguage === "zh" ? "音频预览" : "Audio preview",
    video: currentLanguage === "zh" ? "视频预览" : "Video preview",
    unsupported: currentLanguage === "zh" ? "暂不支持预览" : "Preview not supported",
  };

  if (previewKind !== "text" && previewKind !== "markdown") {
    return labelMap[previewKind] || mimeType || (currentLanguage === "zh" ? "閺傚洣娆㈡０鍕潔" : "File preview");
  }

  return currentLanguage === "zh"
    ? `${Number(file.line_count || 0)} 行${file.truncated ? " / 预览已截断" : ""}`
    : `${Number(file.line_count || 0)} lines${file.truncated ? " / preview truncated" : ""}`;
}

function syncWorkspaceCodeRenderToggle() {
  if (!workspaceCodeRenderToggle) return;
  const isMarkdown = isMarkdownWorkspaceFile(currentWorkspaceFile);
  workspaceCodeRenderToggle.hidden = !isMarkdown;
  if (!isMarkdown) {
    workspaceCodeRenderToggle.textContent = currentLanguage === "zh" ? "渲染" : "Render";
    workspaceCodeRenderToggle.classList.remove("is-active");
    return;
  }
  const isRendered = workspaceCodeRenderMode === "rendered";
  workspaceCodeRenderToggle.textContent = currentLanguage === "zh"
    ? (isRendered ? "源码" : "渲染")
    : (isRendered ? "Source" : "Render");
  workspaceCodeRenderToggle.classList.toggle("is-active", isRendered);
}

function workspaceCodeCanEdit(file = currentWorkspaceFile) {
  if (!file) return false;
  const previewKind = workspaceFilePreviewKind(file);
  return previewKind === "text" || previewKind === "markdown";
}

function workspaceTabName(path) {
  const normalized = String(path || "").replace(/\\/g, "/");
  return normalized.split("/").pop() || normalized || (currentLanguage === "zh" ? "\u672a\u547d\u540d" : "Untitled");
}

function workspaceFileDisplayContent(file) {
  if (!file) return "";
  const path = String(file.path || "");
  if (path && workspaceDraftCache.has(path)) {
    return String(workspaceDraftCache.get(path) ?? "");
  }
  return String(file.content || "");
}

function cacheWorkspaceFile(file) {
  const path = String(file?.path || "").trim();
  if (!path) return;
  if (!workspaceCodeCanEdit(file)) {
    workspaceFileTextCache.delete(path);
    workspaceSymbolIndexCache.delete(path);
    return;
  }
  const cached = {
    path,
    name: String(file?.name || ""),
    language: workspaceEditorLanguage(file),
    content: String(file?.content || ""),
    preview_kind: workspaceFilePreviewKind(file),
    line_count: Number(file?.line_count || 0),
    truncated: Boolean(file?.truncated),
  };
  workspaceFileTextCache.set(path, cached);
  workspaceSymbolIndexCache.delete(path);
}

function workspaceAllFileEntries() {
  return workspaceFlattenFiles(bootstrapData?.workspace_browser?.entries || [], []);
}

async function openWorkspaceFileAt(path, lineNumber = null, column = null) {
  const nextPath = String(path || "").trim();
  if (!nextPath) return;
  captureMessageScrollPosition();
  activeWorkspaceFilePath = nextPath;
  expandWorkspacePathAncestors(nextPath);
  ensureCodePanelVisible();
  workspacePendingReveal = lineNumber
    ? {
        path: nextPath,
        lineNumber: Math.max(1, Number(lineNumber || 1)),
        column: Math.max(1, Number(column || 1)),
      }
    : null;
  renderWorkspaceTree(bootstrapData?.workspace_browser || null);
  await loadWorkspaceFile(nextPath, { preservePanelVisibility: false });
  requestAnimationFrame(() => restoreMessageScrollPosition());
}

function syncWorkspaceDraftForCurrentFile() {
  if (!currentWorkspaceFile?.path || !workspaceCodeCanEdit(currentWorkspaceFile)) return;
  const nextValue = workspaceEditorText();
  const baseValue = String(currentWorkspaceFile.content || "");
  if (nextValue === baseValue) {
    workspaceDraftCache.delete(currentWorkspaceFile.path);
  } else {
    workspaceDraftCache.set(currentWorkspaceFile.path, nextValue);
  }
}

function rememberWorkspaceRenderMode(file = currentWorkspaceFile) {
  if (!file?.path || !isMarkdownWorkspaceFile(file)) return;
  workspaceRenderModeByPath.set(file.path, workspaceCodeRenderMode === "rendered" ? "rendered" : "source");
}

function renderWorkspaceTabs() {
  if (!workspaceCodeTabs) return;
  if (!workspaceOpenTabs.length) {
    workspaceCodeTabs.innerHTML = "";
    workspaceCodeTabs.hidden = true;
    return;
  }

  workspaceCodeTabs.hidden = false;
  workspaceCodeTabs.innerHTML = workspaceOpenTabs
    .map((path) => {
      const active = String(currentWorkspaceFile?.path || "") === path;
      const dirty = workspaceDraftCache.has(path);
      const label = workspaceTabName(path);
      return `
        <div class="workspace-code-tab${active ? " is-active" : ""}" data-workspace-tab="${escapeHtml(path)}">
          <button class="workspace-code-tab-main" type="button" data-workspace-tab-open="${escapeHtml(path)}" title="${escapeHtml(path)}">
            <span class="workspace-code-tab-label">${escapeHtml(label)}</span>
            <span class="workspace-code-tab-dirty${dirty ? " is-visible" : ""}" aria-hidden="true"></span>
          </button>
          <button class="workspace-code-tab-close" type="button" data-workspace-tab-close="${escapeHtml(path)}" aria-label="Close file" title="Close file">
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="m8 8 8 8"></path>
              <path d="m16 8-8 8"></path>
            </svg>
          </button>
        </div>
      `;
    })
    .join("");

  workspaceCodeTabs.querySelectorAll("[data-workspace-tab-open]").forEach((button) => {
    button.addEventListener("click", async () => {
      const path = button.getAttribute("data-workspace-tab-open") || "";
      if (!path) return;
      if (String(currentWorkspaceFile?.path || "") === path) return;
      activeWorkspaceFilePath = path;
      expandWorkspacePathAncestors(path);
      renderWorkspaceTree(bootstrapData?.workspace_browser || null);
      try {
        await loadWorkspaceFile(path);
      } catch (error) {
        console.error(error);
        showToast(appErrorMessage(error, "workspace", "toastSendFailed"));
      }
    });
  });

  workspaceCodeTabs.querySelectorAll("[data-workspace-tab-close]").forEach((button) => {
    button.addEventListener("click", async (event) => {
      event.stopPropagation();
      const path = button.getAttribute("data-workspace-tab-close") || "";
      if (!path) return;
      const wasActive = String(currentWorkspaceFile?.path || "") === path;
      workspaceOpenTabs = workspaceOpenTabs.filter((entry) => entry !== path);
      workspaceDraftCache.delete(path);
      workspaceRenderModeByPath.delete(path);
      workspaceMonacoViewStateCache.delete(path);
      if (!wasActive) {
        renderWorkspaceTabs();
        return;
      }

      const nextPath = workspaceOpenTabs[workspaceOpenTabs.length - 1] || "";
      if (!nextPath) {
        activeWorkspaceFilePath = null;
        renderWorkspaceFile(null);
        renderWorkspaceTree(bootstrapData?.workspace_browser || null);
        return;
      }

      activeWorkspaceFilePath = nextPath;
      renderWorkspaceTree(bootstrapData?.workspace_browser || null);
      try {
        await loadWorkspaceFile(nextPath);
      } catch (error) {
        console.error(error);
        renderWorkspaceFile(null);
        showToast(appErrorMessage(error, "workspace", "toastSendFailed"));
      }
    });
  });
}

function renderWorkspaceBreadcrumbs(file = currentWorkspaceFile) {
  if (!workspaceCodeBreadcrumbs) return;
  if (!file?.path) {
    workspaceCodeBreadcrumbs.innerHTML = "";
    workspaceCodeBreadcrumbs.hidden = true;
    return;
  }
  const parts = String(file.path).replace(/\\/g, "/").split("/").filter(Boolean);
  workspaceCodeBreadcrumbs.hidden = false;
  workspaceCodeBreadcrumbs.innerHTML = parts
    .map((part, index) => {
      const isLast = index === parts.length - 1;
      return `<span class="workspace-code-breadcrumb${isLast ? " is-current" : ""}">${escapeHtml(part)}</span>`;
    })
    .join('<span class="workspace-code-breadcrumb-sep" aria-hidden="true">/</span>');
}

function runWorkspaceEditorAction(actionId) {
  if (!workspaceMonacoEditor || !actionId) return;
  workspaceMonacoEditor.focus?.();
  const action = workspaceMonacoEditor.getAction?.(actionId);
  if (action?.run) {
    action.run().catch?.(() => {});
    return;
  }

  const monaco = window.monaco;
  if (monaco?.editor?.trigger) {
    monaco.editor.trigger("tokitai", actionId, null);
  }
}

function runWorkspaceConfigAction(configId) {
  if (!configId) return;
  const config = (runDebugState?.configs || []).find((item) => item.id === configId);
  if (config?.task_type === "preview" && config.file_hint) {
    openWorkspaceFileAt(config.file_hint).catch((error) => {
      console.error(error);
      showToast(appErrorMessage(error, "workspace", "toastSendFailed"));
    });
    return;
  }
  if (config && config.available === false) {
    showToast(formatRunDependencyMessage(config));
    return;
  }
  runRunDebugAction("start", configId).catch((error) => {
    console.error(error);
    showToast(appErrorMessage(error, "workspace", "toastSendFailed"));
  });
}

function syncWorkspaceCodeToolbar() {
  const enabled = Boolean(currentWorkspaceFile && workspaceMonacoEditor);
  [
    workspaceCodeSearchButton,
    workspaceCodeReplaceButton,
    workspaceCodeLineButton,
    workspaceCodeSymbolsButton,
    workspaceCodeReferencesButton,
    workspaceCodeRenameButton,
  ].forEach((button) => {
    if (!button) return;
    button.disabled = !enabled;
    button.setAttribute("aria-disabled", enabled ? "false" : "true");
  });
}

function workspaceEditorNodes() {
  return {
    editor: document.getElementById("workspace-code-editor"),
    highlight: document.getElementById("workspace-code-highlight"),
    gutter: document.getElementById("workspace-code-gutter"),
    completions: document.getElementById("workspace-code-completions"),
    pane: document.getElementById("workspace-code-editor-pane"),
  };
}

function workspaceEditorLanguage(file = currentWorkspaceFile) {
  return normalizeCodeLanguage(file?.language || file?.path || "text");
}

function workspaceEditorText() {
  if (workspaceMonacoEditor) {
    return String(workspaceMonacoEditor.getValue() ?? currentWorkspaceFile?.content ?? "");
  }
  const { editor } = workspaceEditorNodes();
  return String(editor?.value ?? currentWorkspaceFile?.content ?? "");
}

function workspaceEditorSelection(editor = workspaceEditorNodes().editor) {
  if (workspaceMonacoEditor && typeof window.monaco !== "undefined") {
    const selection = workspaceMonacoEditor.getSelection();
    const model = workspaceMonacoEditor.getModel();
    if (selection && model) {
      const selectionStart = model.getOffsetAt({
        lineNumber: selection.startLineNumber,
        column: selection.startColumn,
      });
      const selectionEnd = model.getOffsetAt({
        lineNumber: selection.endLineNumber,
        column: selection.endColumn,
      });
      return { selectionStart, selectionEnd };
    }
  }
  const selectionStart = Number(editor?.selectionStart ?? 0);
  const selectionEnd = Number(editor?.selectionEnd ?? selectionStart);
  return { selectionStart, selectionEnd };
}

function workspaceMonacoModulePath(language) {
  const normalized = normalizeCodeLanguage(language);
  const moduleByLanguage = {
    c: "cpp",
    cpp: "cpp",
    csharp: "csharp",
    css: "css",
    go: "go",
    html: "html",
    ini: "ini",
    java: "java",
    javascript: "javascript",
    markdown: "markdown",
    python: "python",
    rust: "rust",
    shell: "shell",
    typescript: "typescript",
    yaml: "yaml",
  };
  return moduleByLanguage[normalized] || null;
}

function workspaceMonacoLanguageConfig(language) {
  const normalized = normalizeCodeLanguage(language);
  if (!normalized || normalized === "text") {
    return { languageId: "plaintext", sourceLanguage: "text" };
  }
  if (normalized === "typescript") {
    return { languageId: "typescript", sourceLanguage: "typescript" };
  }
  if (normalized === "shell") {
    return { languageId: "shell", sourceLanguage: "shell" };
  }
  if (normalized === "json") {
    return { languageId: "json", sourceLanguage: "json" };
  }
  return { languageId: normalized || "plaintext", sourceLanguage: normalized || "text" };
}

function loadMonaco() {
  if (typeof window === "undefined" || typeof window.require !== "function") {
    return Promise.reject(new Error("Monaco loader is unavailable"));
  }
  if (workspaceMonacoLoaderPromise) return workspaceMonacoLoaderPromise;
  workspaceMonacoLoaderPromise = new Promise((resolve, reject) => {
    window.require(["vs/editor/editor.main"], () => {
      if (window.monaco) {
        resolve(window.monaco);
      } else {
        reject(new Error("Monaco failed to initialize"));
      }
    }, reject);
  });
  return workspaceMonacoLoaderPromise;
}

async function ensureMonacoLanguage(language) {
  const monaco = await loadMonaco();
  if (!workspaceMonacoThemeReady && monaco?.editor?.defineTheme) {
    monaco.editor.defineTheme("tokitai-warm", {
      base: "vs-dark",
      inherit: true,
      rules: [
        { token: "comment", foreground: "9b8f81", fontStyle: "italic" },
        { token: "keyword", foreground: "f59e63" },
        { token: "string", foreground: "d9c4b0" },
        { token: "number", foreground: "e6b17a" },
        { token: "type", foreground: "9cc1ff" },
        { token: "function", foreground: "ffd3ad" },
        { token: "variable", foreground: "f4ede5" },
      ],
      colors: {
        "editor.background": "#171513",
        "editor.foreground": "#f4ede5",
        "editorLineNumber.foreground": "#7f7469",
        "editorLineNumber.activeForeground": "#f4ede5",
        "editorCursor.foreground": "#f4ede5",
        "editor.selectionBackground": "#f59e6340",
        "editor.inactiveSelectionBackground": "#f59e6322",
        "editor.lineHighlightBackground": "#ffffff05",
        "editor.lineHighlightBorder": "#00000000",
        "editorIndentGuide.background1": "#ffffff12",
        "editorIndentGuide.activeBackground1": "#f59e6340",
        "scrollbar.shadow": "#00000000",
        "scrollbarSlider.background": "#ffffff18",
        "scrollbarSlider.hoverBackground": "#ffffff28",
        "scrollbarSlider.activeBackground": "#ffffff36",
      },
    });
    workspaceMonacoThemeReady = true;
  }
  const { languageId, sourceLanguage } = workspaceMonacoLanguageConfig(language);
  const moduleName = workspaceMonacoModulePath(sourceLanguage);
  if (!moduleName || workspaceMonacoRegisteredLanguages.has(languageId)) {
    return { monaco, languageId };
  }

  await new Promise((resolve) => {
    const primaryPath = `vs/basic-languages/${moduleName}/${moduleName}`;
    const fallbackPath = `vs/basic-languages/${sourceLanguage}/${sourceLanguage}`;
    window.require([primaryPath], (moduleExports) => {
      const exported = moduleExports || {};
      const languageDef = exported.language || exported.default?.language;
      const conf = exported.conf || exported.default?.conf;
      const alreadyRegistered = monaco.languages.getLanguages().some((entry) => entry.id === languageId);
      if (!alreadyRegistered) {
        monaco.languages.register({ id: languageId });
      }
      if (languageDef) {
        monaco.languages.setMonarchTokensProvider(languageId, languageDef);
      }
      if (conf) {
        monaco.languages.setLanguageConfiguration(languageId, conf);
      }
      workspaceMonacoRegisteredLanguages.add(languageId);
      resolve();
    }, () => {
      if (primaryPath === fallbackPath) {
        resolve();
        return;
      }
      window.require([fallbackPath], (moduleExports) => {
        const exported = moduleExports || {};
        const languageDef = exported.language || exported.default?.language;
        const conf = exported.conf || exported.default?.conf;
        const alreadyRegistered = monaco.languages.getLanguages().some((entry) => entry.id === languageId);
        if (!alreadyRegistered) {
          monaco.languages.register({ id: languageId });
        }
        if (languageDef) {
          monaco.languages.setMonarchTokensProvider(languageId, languageDef);
        }
        if (conf) {
          monaco.languages.setLanguageConfiguration(languageId, conf);
        }
        workspaceMonacoRegisteredLanguages.add(languageId);
        resolve();
      }, () => resolve());
    });
  });

  return { monaco, languageId };
}

async function ensureWorkspaceMonacoCompletionProvider(languageId) {
  const monaco = await loadMonaco();
  if (!languageId || workspaceMonacoCompletionProviders.has(languageId)) return;

  monaco.languages.registerCompletionItemProvider(languageId, {
    triggerCharacters: [".", "_", "$", ":", "/", "-", "@", "\"", "'"],
    provideCompletionItems: async (model, position) => {
      if (!currentWorkspaceFile || model !== workspaceMonacoModel) {
        return { suggestions: [] };
      }

      const content = model.getValue();
      const offset = model.getOffsetAt(position);
      const tokenInfo = workspaceCompletionToken(content, offset);
      const query = tokenInfo.tokenPrefix.trim();
      const localItems = workspaceLocalCompletionItems(query, content, workspaceEditorLanguage());
      const merged = [];
      const seen = new Set();
      const addItem = (item, kind = monaco.languages.CompletionItemKind.Text) => {
        const insertText = String(item?.insert_text || item?.label || "").trim();
        if (!insertText) return;
        const dedupeKey = insertText.toLowerCase();
        if (seen.has(dedupeKey)) return;
        seen.add(dedupeKey);
        merged.push({
          label: String(item?.label || insertText),
          insertText,
          detail: String(item?.detail || ""),
          kind,
          range: new monaco.Range(
            position.lineNumber,
            tokenInfo.columnNumber - tokenInfo.tokenPrefix.length,
            position.lineNumber,
            tokenInfo.columnNumber,
          ),
          sortText: String(item?.source || "1"),
        });
      };

      localItems.forEach((item) => addItem(item, monaco.languages.CompletionItemKind.Text));
      if (query.length >= 2) {
        try {
          const prefixContext = content.slice(Math.max(0, tokenInfo.tokenStart - 1200), tokenInfo.tokenEnd);
          const suffixContext = content.slice(tokenInfo.tokenEnd, Math.min(content.length, tokenInfo.tokenEnd + 800));
          const response = await hostClient.workspace.completeFile({
            path: currentWorkspaceFile?.path || "",
            language: workspaceEditorLanguage(),
            token_prefix: tokenInfo.tokenPrefix,
            prefix: prefixContext,
            suffix: suffixContext,
            cursor_line: tokenInfo.lineNumber || 1,
            cursor_column: tokenInfo.columnNumber || 1,
          });
          if (response.ok) {
            const payload = await response.json();
            const remoteItems = Array.isArray(payload?.data?.items) ? payload.data.items : [];
            remoteItems.forEach((item) => addItem(item, monaco.languages.CompletionItemKind.Snippet));
          }
        } catch (_error) {
          // Ignore remote completion failures and keep local suggestions.
        }
      }

      return { suggestions: merged };
    },
  });

  workspaceMonacoCompletionProviders.add(languageId);
}

async function ensureWorkspaceMonacoLanguageProviders(languageId) {
  const monaco = await loadMonaco();
  if (!languageId || workspaceMonacoDefinitionProviders.has(languageId)) return;

  monaco.languages.registerDocumentSymbolProvider(languageId, {
    provideDocumentSymbols(model) {
      const text = String(model.getValue() || "");
      const lines = text.split("\n");
      const symbols = [];
      const symbolKind = monaco.languages.SymbolKind;
      const patterns = [
        { regex: /^\s*function\s+([A-Za-z_$][\w$]*)/, kind: symbolKind.Function },
        { regex: /^\s*(?:export\s+)?(?:async\s+)?function\s+([A-Za-z_$][\w$]*)/, kind: symbolKind.Function },
        { regex: /^\s*(?:const|let|var)\s+([A-Za-z_$][\w$]*)\s*=\s*(?:async\s*)?\(/, kind: symbolKind.Function },
        { regex: /^\s*class\s+([A-Za-z_$][\w$]*)/, kind: symbolKind.Class },
        { regex: /^\s*(?:export\s+)?class\s+([A-Za-z_$][\w$]*)/, kind: symbolKind.Class },
        { regex: /^\s*def\s+([A-Za-z_][\w]*)/, kind: symbolKind.Function },
        { regex: /^\s*(?:pub\s+)?fn\s+([A-Za-z_][\w]*)/, kind: symbolKind.Function },
        { regex: /^\s*(?:pub\s+)?struct\s+([A-Za-z_][\w]*)/, kind: symbolKind.Struct },
        { regex: /^\s*(?:pub\s+)?enum\s+([A-Za-z_][\w]*)/, kind: symbolKind.Enum },
      ];

      lines.forEach((line, index) => {
        for (const pattern of patterns) {
          const match = line.match(pattern.regex);
          if (!match) continue;
          const name = match[1];
          const startColumn = Math.max(1, line.indexOf(name) + 1);
          const range = new monaco.Range(index + 1, 1, index + 1, Math.max(1, line.length + 1));
          const selectionRange = new monaco.Range(index + 1, startColumn, index + 1, startColumn + name.length);
          symbols.push({
            name,
            detail: "",
            kind: pattern.kind,
            range,
            selectionRange,
          });
          break;
        }
      });

      return symbols;
    },
  });

  monaco.languages.registerDefinitionProvider(languageId, {
    provideDefinition: async (model, position) => {
      const word = model.getWordAtPosition(position);
      if (!word?.word) return [];
      const matches = model.findMatches(word.word, true, true, false, null, true) || [];
      const currentLine = position.lineNumber;
      const localDefinition = matches.find((entry) => entry.range.startLineNumber !== currentLine) || matches[0] || null;
      const currentPath = String(currentWorkspaceFile?.path || "");
      const crossFile = workspaceFindDefinitionTarget(word.word, currentPath);

      if (crossFile && crossFile.path && crossFile.path !== currentPath) {
        if (!workspaceFileTextCache.has(crossFile.path)) {
          try {
            const response = await hostClient.workspace.openFile(crossFile.path);
            if (response.ok) {
              const payload = await response.json();
              cacheWorkspaceFile(payload?.data?.file || null);
            }
          } catch (_error) {
            // Ignore cache warmup failures for definition jumps.
          }
        }
        const uri = monaco.Uri.parse(`file:///${String(crossFile.path).replace(/\\/g, "/").replace(/^\/+/, "")}`);
        return [{
          uri,
          range: new monaco.Range(
            crossFile.lineNumber,
            crossFile.column,
            crossFile.lineNumber,
            crossFile.column + String(crossFile.name || word.word).length,
          ),
        }];
      }

      if (!localDefinition) return [];
      return [{
        uri: model.uri,
        range: localDefinition.range,
      }];
    },
  });

  if (!workspaceMonacoHoverProviders.has(languageId)) {
    monaco.languages.registerHoverProvider(languageId, {
      provideHover(model, position) {
        const word = model.getWordAtPosition(position);
        if (!word?.word) return null;
        const target = workspaceFindDefinitionTarget(word.word, String(currentWorkspaceFile?.path || ""));
        if (!target) return null;
        const locationText = target.path === String(currentWorkspaceFile?.path || "")
          ? (currentLanguage === "zh" ? "瑜版挸澧犻弬鍥︽" : "Current file")
          : target.path;
        return {
          range: new monaco.Range(position.lineNumber, word.startColumn, position.lineNumber, word.endColumn),
          contents: [
            { value: `**${word.word}**` },
            { value: target.lineText ? `\`${target.lineText}\`` : "" },
            { value: currentLanguage === "zh" ? `鐎规矮绠熸担宥囩枂: ${locationText}:${target.lineNumber}` : `Definition: ${locationText}:${target.lineNumber}` },
          ].filter((entry) => entry.value),
        };
      },
    });
    workspaceMonacoHoverProviders.add(languageId);
  }

  workspaceMonacoDefinitionProviders.add(languageId);
}

function disposeWorkspaceMonaco() {
  workspaceMonacoRenderToken += 1;
  workspaceCodeResizeObserver?.disconnect?.();
  workspaceCodeResizeObserver = null;
  if (workspaceMonacoEditor && workspaceMonacoModel) {
    const pathKey = String(currentWorkspaceFile?.path || workspaceMonacoModel.uri?.path || workspaceMonacoModel.uri?.toString() || "");
    if (pathKey) {
      try {
        workspaceMonacoViewStateCache.set(pathKey, workspaceMonacoEditor.saveViewState?.() || null);
      } catch (_error) {
        // Ignore saveViewState failures.
      }
    }
  }
  try {
    workspaceMonacoEditor?.dispose?.();
  } catch (_error) {
    // Ignore disposal failures.
  }
  workspaceMonacoEditor = null;
  workspaceMonacoModel = null;
  workspaceMonacoLanguageId = "";
  syncWorkspaceCodeToolbar();
}

function syncWorkspaceLauncherVisibility() {
  if (!workspaceLauncher) return;
  const shouldShow = preserveWorkspaceSlotWhenCodeClosed && !currentWorkspaceFile && !isWorkspaceCodeOpen;
  workspaceLauncher.hidden = !shouldShow;
}

function scheduleWorkspaceMonacoLayout() {
  if (!workspaceMonacoEditor) return;
  window.requestAnimationFrame(() => workspaceMonacoEditor?.layout?.());
  window.requestAnimationFrame(() => {
    window.requestAnimationFrame(() => workspaceMonacoEditor?.layout?.());
  });
  window.setTimeout(() => workspaceMonacoEditor?.layout?.(), 120);
}

async function mountWorkspaceMonaco(host, file) {
  const renderToken = ++workspaceMonacoRenderToken;
  const { monaco, languageId } = await ensureMonacoLanguage(workspaceEditorLanguage(file));
  await ensureWorkspaceMonacoCompletionProvider(languageId);
  await ensureWorkspaceMonacoLanguageProviders(languageId);
  if (renderToken !== workspaceMonacoRenderToken || !host || !file) return;

  disposeWorkspaceMonaco();
  workspaceMonacoRenderToken = renderToken;

  const fileKey = String(file.path || file.name || "");
  const modelUri = monaco.Uri.parse(`file:///${String(fileKey || "workspace.txt").replace(/\\/g, "/").replace(/^\/+/, "")}`);
  const cachedModel = fileKey ? workspaceMonacoModelCache.get(fileKey) : null;
  if (cachedModel && !cachedModel.isDisposed?.()) {
    workspaceMonacoModel = cachedModel;
    if (String(workspaceMonacoModel.getValue?.() || "") !== String(file.content || "")) {
      workspaceMonacoModel.setValue(String(file.content || ""));
    }
    if (workspaceMonacoModel.getLanguageId?.() !== languageId && monaco?.editor?.setModelLanguage) {
      monaco.editor.setModelLanguage(workspaceMonacoModel, languageId);
    }
  } else {
    workspaceMonacoModel = monaco.editor.createModel(String(file.content || ""), languageId, modelUri);
    if (fileKey) {
      workspaceMonacoModelCache.set(fileKey, workspaceMonacoModel);
    }
  }
  workspaceMonacoLanguageId = languageId;

  workspaceMonacoEditor = monaco.editor.create(host, {
    model: workspaceMonacoModel,
    automaticLayout: true,
    theme: "tokitai-warm",
    scrollBeyondLastLine: false,
    minimap: { enabled: false },
    lineNumbers: "on",
    glyphMargin: false,
    folding: true,
    foldingHighlight: true,
    lineDecorationsWidth: 10,
    lineNumbersMinChars: 4,
    tabSize: 2,
    insertSpaces: true,
    fontSize: 13,
    lineHeight: 21,
    fontFamily: '"SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace',
    wordWrap: "off",
    renderWhitespace: "selection",
    bracketPairColorization: { enabled: true },
    guides: {
      indentation: true,
      bracketPairs: true,
      highlightActiveIndentation: true,
    },
    smoothScrolling: true,
    cursorBlinking: "solid",
    overviewRulerBorder: false,
    scrollbar: {
      verticalScrollbarSize: 8,
      horizontalScrollbarSize: 8,
      useShadows: false,
      alwaysConsumeMouseWheel: false,
    },
    padding: {
      top: 16,
      bottom: 18,
    },
  });

  workspaceCodeResizeObserver?.disconnect?.();
  workspaceCodeResizeObserver = new ResizeObserver(() => {
    workspaceMonacoEditor?.layout?.();
  });
  workspaceCodeResizeObserver.observe(host);

  workspaceMonacoEditor.onDidChangeModelContent(() => {
    const nextValue = workspaceMonacoEditor?.getValue() ?? "";
    if (currentWorkspaceFile?.path && workspaceCodeCanEdit(currentWorkspaceFile)) {
      const baseValue = String(currentWorkspaceFile?.content || "");
      if (nextValue === baseValue) {
        workspaceDraftCache.delete(currentWorkspaceFile.path);
      } else {
        workspaceDraftCache.set(currentWorkspaceFile.path, nextValue);
      }
      workspaceFileTextCache.set(currentWorkspaceFile.path, {
        path: currentWorkspaceFile.path,
        name: String(currentWorkspaceFile?.name || ""),
        language: workspaceEditorLanguage(),
        content: nextValue,
        preview_kind: workspaceFilePreviewKind(currentWorkspaceFile),
        line_count: nextValue.split("\n").length,
        truncated: false,
      });
      workspaceSymbolIndexCache.forEach((_value, key) => {
        if (String(key).startsWith(`${currentWorkspaceFile.path}::`)) {
          workspaceSymbolIndexCache.delete(key);
        }
      });
      renderWorkspaceTabs();
    }
    markWorkspaceEditorDirty(nextValue !== String(currentWorkspaceFile?.content || ""));
    scheduleWorkspaceDiagnostics();
  });

  workspaceMonacoEditor.onDidFocusEditorText(() => {
    workspaceEditorComposing = false;
  });

  workspaceMonacoEditor.onDidChangeCursorSelection((event) => {
    if (!event?.selection?.isEmpty?.()) return;
    const model = workspaceMonacoEditor?.getModel?.();
    const position = event.selection.getPosition?.();
    const word = model?.getWordAtPosition?.(position);
    if (!word?.word) {
      if (workspaceCodeMeta) {
        workspaceCodeMeta.textContent = workspaceFileMetaText(currentWorkspaceFile);
      }
      return;
    }
    const target = workspaceFindDefinitionTarget(word.word, String(currentWorkspaceFile?.path || ""));
    if (target && target.path && target.path !== String(currentWorkspaceFile?.path || "")) {
      workspaceCodeMeta.textContent = `${workspaceFileMetaText(currentWorkspaceFile)} / ${target.path}:${target.lineNumber}`;
    } else if (workspaceCodeMeta) {
      workspaceCodeMeta.textContent = workspaceFileMetaText(currentWorkspaceFile);
    }
  });

  workspaceMonacoEditor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => {
    saveWorkspaceFile().catch((error) => {
      console.error(error);
      showToast(error?.message || t("toastSendFailed"));
    });
  });

  workspaceMonacoEditor.addCommand(monaco.KeyCode.F12, () => {
    runWorkspaceEditorAction("editor.action.revealDefinition");
  });

  workspaceMonacoEditor.addCommand(monaco.KeyMod.Shift | monaco.KeyCode.F12, () => {
    runWorkspaceFindReferences().catch(() => {});
  });

  workspaceMonacoEditor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.F2, () => {
    runWorkspaceRenameSymbol().catch(() => {});
  });

  workspaceMonacoEditor.onDidCompositionStart?.(() => {
    workspaceEditorComposing = true;
  });

  workspaceMonacoEditor.onDidCompositionEnd?.(() => {
    workspaceEditorComposing = false;
  });

  workspaceMonacoEditor.onDidAttemptReadOnlyEdit?.(() => {
    showToast(currentLanguage === "zh" ? "瑜版挸澧犵憴鍡楁禈娑撳秴褰茬紓鏍帆" : "This view is read only");
  });

  window.requestAnimationFrame(() => {
    scheduleWorkspaceMonacoLayout();
    const cachedViewState = workspaceMonacoViewStateCache.get(fileKey);
    if (cachedViewState) {
      try {
        workspaceMonacoEditor.restoreViewState?.(cachedViewState);
      } catch (_error) {
        // Ignore stale view states.
      }
    }
    const shouldFocusEditor = Boolean(workspacePendingReveal && workspacePendingReveal.path === fileKey);
    if (shouldFocusEditor) {
      const targetLine = Math.max(1, Number(workspacePendingReveal.lineNumber || 1));
      const targetColumn = Math.max(1, Number(workspacePendingReveal.column || 1));
      workspaceMonacoEditor?.revealPositionInCenter?.({ lineNumber: targetLine, column: targetColumn });
      workspaceMonacoEditor?.setPosition?.({ lineNumber: targetLine, column: targetColumn });
      workspacePendingReveal = null;
    }
    if (shouldFocusEditor) {
      workspaceMonacoEditor?.focus?.();
    }
  });
  cacheWorkspaceFile(file);
  scheduleWorkspaceDiagnostics();
  syncWorkspaceCodeToolbar();
}

function buildWorkspaceEditorGutter(text) {
  const lineCount = Math.max(1, String(text || "").replace(/\r\n/g, "\n").split("\n").length);
  let html = "";
  for (let line = 1; line <= lineCount; line += 1) {
    html += `<div class="workspace-code-gutter-line">${line}</div>`;
  }
  return html;
}

function workspaceCompletionToken(value, cursor) {
  const text = String(value || "");
  const safeCursor = Math.max(0, Math.min(cursor, text.length));
  const before = text.slice(0, safeCursor);
  const after = text.slice(safeCursor);
  const lineStart = before.lastIndexOf("\n") + 1;
  const lineEndIndex = after.indexOf("\n");
  const lineEnd = lineEndIndex >= 0 ? safeCursor + lineEndIndex : text.length;
  const lineBefore = before.slice(lineStart);
  const tokenMatch = lineBefore.match(/[A-Za-z_$][A-Za-z0-9_$]*$/);
  const tokenPrefix = tokenMatch ? tokenMatch[0] : "";
  const tokenStart = tokenMatch ? safeCursor - tokenPrefix.length : safeCursor;
  return {
    tokenPrefix,
    tokenStart,
    tokenEnd: safeCursor,
    lineText: text.slice(lineStart, lineEnd),
    lineNumber: before.split("\n").length,
    columnNumber: safeCursor - lineStart + 1,
    lineStart,
    lineEnd,
    lineBefore,
    before,
    after,
  };
}

function workspaceLocalCompletionItems(prefix, text, language) {
  const query = String(prefix || "").trim();
  if (query.length < 2) return [];
  const normalizedQuery = query.toLowerCase();
  const lang = normalizeCodeLanguage(language);
  const group = CODE_HIGHLIGHT_GROUPS[lang] || CODE_HIGHLIGHT_GROUPS.javascript;
  const candidates = new Map();

  const pushCandidate = (value, detail = "") => {
    const candidate = String(value || "").trim();
    if (!candidate || candidate.toLowerCase() === normalizedQuery) return;
    if (!candidate.toLowerCase().startsWith(normalizedQuery)) return;
    const score = candidates.get(candidate);
    candidates.set(candidate, (score || 0) + 1 + (detail ? 0.1 : 0));
  };

  [...(group.keywords || []), ...(group.builtins || [])].forEach((item) => pushCandidate(item, "syntax"));
  String(text || "")
    .slice(0, 50000)
    .match(/[A-Za-z_$][A-Za-z0-9_$]{2,}/g)
    ?.forEach((token) => pushCandidate(token, "identifier"));

  return Array.from(candidates.entries())
    .sort((left, right) => right[1] - left[1] || left[0].length - right[0].length || left[0].localeCompare(right[0]))
    .slice(0, 8)
    .map(([label]) => ({
      label,
      insert_text: label,
      detail: "",
      source: "syntax",
    }));
}

function workspaceIndexSymbolsForContent(path, language, text) {
  const cacheKey = `${path}::${language}::${text.length}`;
  const cached = workspaceSymbolIndexCache.get(cacheKey);
  if (cached) return cached;

  const normalizedLanguage = normalizeCodeLanguage(language);
  const lines = String(text || "").split("\n");
  const patterns = [
    { regex: /^\s*(?:export\s+)?(?:async\s+)?function\s+([A-Za-z_$][\w$]*)/, kind: "function" },
    { regex: /^\s*(?:const|let|var)\s+([A-Za-z_$][\w$]*)\s*=\s*(?:async\s*)?\(/, kind: "function" },
    { regex: /^\s*(?:export\s+)?class\s+([A-Za-z_$][\w$]*)/, kind: "class" },
    { regex: /^\s*def\s+([A-Za-z_][\w]*)/, kind: "function" },
    { regex: /^\s*(?:pub\s+)?fn\s+([A-Za-z_][\w]*)/, kind: "function" },
    { regex: /^\s*(?:pub\s+)?struct\s+([A-Za-z_][\w]*)/, kind: "struct" },
    { regex: /^\s*(?:pub\s+)?enum\s+([A-Za-z_][\w]*)/, kind: "enum" },
    { regex: /^\s*(?:pub\s+)?trait\s+([A-Za-z_][\w]*)/, kind: "interface" },
  ];
  const symbols = [];

  lines.forEach((line, index) => {
    for (const pattern of patterns) {
      const match = line.match(pattern.regex);
      if (!match) continue;
      const name = match[1];
      const column = Math.max(1, line.indexOf(name) + 1);
      symbols.push({
        name,
        kind: pattern.kind,
        path,
        language: normalizedLanguage,
        lineNumber: index + 1,
        column,
        lineText: line.trim(),
      });
      break;
    }
  });

  workspaceSymbolIndexCache.set(cacheKey, symbols);
  return symbols;
}

function workspaceFindDefinitionTarget(word, currentPath = String(currentWorkspaceFile?.path || "")) {
  const needle = String(word || "").trim();
  if (!needle) return null;

  const activeSources = [];
  const currentText = workspaceEditorText();
  if (currentPath && currentText) {
    activeSources.push({
      path: currentPath,
      language: workspaceEditorLanguage(),
      content: currentText,
    });
  }

  workspaceFileTextCache.forEach((file, path) => {
    if (!file || !path || path === currentPath) return;
    activeSources.push(file);
  });

  for (const source of activeSources) {
    const symbols = workspaceIndexSymbolsForContent(source.path, source.language, source.content);
    const match = symbols.find((entry) => entry.name === needle);
    if (match) return match;
  }

  return null;
}

function workspaceFindReferenceEntries(word) {
  const needle = String(word || "").trim();
  if (!needle) return [];
  const pattern = new RegExp(`\\b${escapeRegExp(needle)}\\b`, "g");
  const results = [];
  const activePath = String(currentWorkspaceFile?.path || "");
  const activeText = workspaceEditorText();

  const pushMatches = (path, content) => {
    const text = String(content || "");
    if (!text) return;
    const lines = text.split("\n");
    lines.forEach((line, index) => {
      pattern.lastIndex = 0;
      let match = pattern.exec(line);
      while (match) {
        results.push({
          path,
          lineNumber: index + 1,
          column: match.index + 1,
          preview: line.trim(),
        });
        match = pattern.exec(line);
      }
    });
  };

  if (activePath && activeText) {
    pushMatches(activePath, activeText);
  }

  workspaceFileTextCache.forEach((file, path) => {
    if (!file || path === activePath) return;
    pushMatches(path, file.content);
  });

  return results.slice(0, 200);
}

function revealWorkspaceReferenceMatches(matches, symbol) {
  workspaceReferenceMatches = Array.isArray(matches) ? matches : [];
  if (!workspaceReferenceMatches.length) {
    showToast(currentLanguage === "zh" ? "未找到引用" : "No references found");
    return;
  }
  const summary = currentLanguage === "zh"
    ? `${symbol}: ${workspaceReferenceMatches.length} 处引用`
    : `${symbol}: ${workspaceReferenceMatches.length} references`;
  showToast(summary);
  if (!reviewStrip) return;
  reviewStrip.hidden = false;
  reviewStrip.innerHTML = `
    <div class="review-strip-head">
      <div class="review-strip-title">${escapeHtml(currentLanguage === "zh" ? "\u7b26\u53f7\u5f15\u7528" : "Symbol References")}</div>
      <div class="review-strip-meta">${escapeHtml(summary)}</div>
    </div>
    <div class="review-file-list review-file-list-collapsed">
      ${workspaceReferenceMatches
        .map((entry) => `
          <article class="review-file-item">
            <button
              class="review-file-chip"
              type="button"
              data-open-workspace-file="${escapeHtml(entry.path)}"
              data-open-workspace-line="${escapeHtml(String(entry.lineNumber))}"
              data-open-workspace-column="${escapeHtml(String(entry.column))}"
            >
              <span class="review-file-main">
                <span class="review-file-path">${escapeHtml(entry.path)}</span>
                <span class="review-file-status">${escapeHtml(`L${entry.lineNumber}:C${entry.column}`)}</span>
              </span>
              <span class="review-file-side">
                <span class="review-file-count">${escapeHtml(entry.preview || "")}</span>
              </span>
            </button>
          </article>
        `)
        .join("")}
    </div>
  `;
  bindTurnInteractionHandlers(reviewStrip);
}

async function runWorkspaceFindReferences() {
  const model = workspaceMonacoEditor?.getModel?.();
  const position = workspaceMonacoEditor?.getPosition?.();
  const word = model?.getWordAtPosition?.(position);
  if (!word?.word) {
    showToast(currentLanguage === "zh" ? "请先将光标放在符号上" : "Place the cursor on a symbol first");
    return;
  }
  const references = workspaceFindReferenceEntries(word.word);
  revealWorkspaceReferenceMatches(references, word.word);
}

async function runWorkspaceRenameSymbol() {
  const model = workspaceMonacoEditor?.getModel?.();
  const position = workspaceMonacoEditor?.getPosition?.();
  const word = model?.getWordAtPosition?.(position);
  if (!word?.word) {
    showToast(currentLanguage === "zh" ? "请先将光标放在符号上" : "Place the cursor on a symbol first");
    return;
  }
  const nextName = window.prompt(
    currentLanguage === "zh" ? `重命名符号 ${word.word}` : `Rename symbol ${word.word}`,
    word.word,
  );
  const normalized = String(nextName || "").trim();
  if (!normalized || normalized === word.word) return;
  const references = workspaceFindReferenceEntries(word.word);
  const currentPath = String(currentWorkspaceFile?.path || "");
  const localReferences = references.filter((entry) => entry.path === currentPath);
  if (!localReferences.length) {
    showToast(currentLanguage === "zh" ? "轻量重命名目前只支持当前文件中的符号" : "Lightweight rename currently supports the active file only");
    return;
  }
  const edit = {
    edits: [
      {
        resource: workspaceMonacoModel.uri,
        textEdit: {
          range: new window.monaco.Range(1, 1, workspaceMonacoModel.getLineCount(), workspaceMonacoModel.getLineMaxColumn(workspaceMonacoModel.getLineCount())),
          text: String(workspaceMonacoModel.getValue() || "").replace(
            new RegExp(`\\b${escapeRegExp(word.word)}\\b`, "g"),
            normalized,
          ),
        },
      },
    ],
  };
  workspaceMonacoEditor.executeEdits("tokitai.rename", edit.edits.map((item) => item.textEdit));
  showToast(
    currentLanguage === "zh"
      ? `已在当前文件中重命名 ${localReferences.length} 处`
      : `Renamed ${localReferences.length} occurrences in current file`,
  );
}

function scheduleWorkspaceDiagnostics() {
  window.clearTimeout(workspaceDiagnosticsTimer);
  workspaceDiagnosticsTimer = window.setTimeout(() => {
    workspaceDiagnosticsTimer = null;
    updateWorkspaceDiagnostics();
  }, 120);
}

async function updateWorkspaceDiagnostics() {
  if (!workspaceMonacoEditor || !workspaceMonacoModel || !window.monaco) return;
  const monaco = window.monaco;
  const text = String(workspaceMonacoModel.getValue() || "");
  const language = workspaceEditorLanguage();
  const markers = [];
  const lines = text.split("\n");
  const bracketPairs = { "(": ")", "[": "]", "{": "}" };
  const closing = new Set(Object.values(bracketPairs));
  const stack = [];

  for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
    const line = lines[lineIndex];
    for (let charIndex = 0; charIndex < line.length; charIndex += 1) {
      const ch = line[charIndex];
      if (bracketPairs[ch]) {
        stack.push({ ch, lineNumber: lineIndex + 1, column: charIndex + 1 });
      } else if (closing.has(ch)) {
        const last = stack[stack.length - 1];
        const expected = last ? bracketPairs[last.ch] : null;
        if (!last || expected !== ch) {
          markers.push({
            severity: monaco.MarkerSeverity.Warning,
            message: currentLanguage === "zh" ? "閹奉剙褰块崣顖濆厴濞屸剝婀佸锝団€橀柊宥咁嚠" : "Bracket may be unmatched",
            message: currentLanguage === "zh" ? "閹奉剙褰块崣顖濆厴濞屸剝婀佸锝団€橀柊宥咁嚠" : "Bracket may be unmatched",
            startColumn: charIndex + 1,
            endLineNumber: lineIndex + 1,
            endColumn: charIndex + 2,
          });
        } else {
          stack.pop();
        }
      }
    }
  }

  stack.slice(-8).forEach((entry) => {
    markers.push({
      severity: monaco.MarkerSeverity.Hint,
      message: currentLanguage === "zh" ? "这个括号尚未闭合" : "This bracket is not yet closed",
      startColumn: entry.column,
      endLineNumber: entry.lineNumber,
      endColumn: entry.column + 1,
    });
  });

  if (language === "python" || language === "yaml") {
    lines.forEach((line, index) => {
      if (/^\t+/.test(line)) {
        markers.push({
          severity: monaco.MarkerSeverity.Info,
          message: currentLanguage === "zh" ? "Tab 缩进可能导致格式不一致" : "Tab indentation may cause inconsistent formatting",
          startLineNumber: index + 1,
          startColumn: 1,
          endLineNumber: index + 1,
          endColumn: Math.min(line.length + 1, 3),
        });
      }
    });
  }

  monaco.editor.setModelMarkers(workspaceMonacoModel, "tokitai-lite", markers);
}

function renderWorkspaceCodeCompletions(items, tokenInfo) {
  const { completions, pane, editor } = workspaceEditorNodes();
  if (!completions || !pane || !editor) return;
  const list = Array.isArray(items) ? items.slice(0, 8) : [];
  if (!list.length) {
    completions.hidden = true;
    completions.innerHTML = "";
    workspaceCompletionState.visible = false;
    workspaceCompletionState.items = [];
    workspaceCompletionState.activeIndex = 0;
    return;
  }

  const nextPrefix = tokenInfo?.tokenPrefix || "";
  const sameQuery = workspaceCompletionState.visible && workspaceCompletionState.tokenPrefix === nextPrefix;
  workspaceCompletionState.visible = true;
  workspaceCompletionState.items = list;
  workspaceCompletionState.activeIndex = sameQuery
    ? Math.max(0, Math.min(workspaceCompletionState.activeIndex, list.length - 1))
    : 0;
  workspaceCompletionState.tokenPrefix = nextPrefix;
  workspaceCompletionState.tokenStart = Number(tokenInfo?.tokenStart || 0);
  workspaceCompletionState.tokenEnd = Number(tokenInfo?.tokenEnd || 0);

  const activeItem = list[workspaceCompletionState.activeIndex] || list[0];
  const itemMarkup = list
    .map((item, index) => {
      const isActive = index === workspaceCompletionState.activeIndex;
      return `
        <button
          class="workspace-code-completion-item${isActive ? " is-active" : ""}"
          type="button"
          data-code-completion-index="${index}"
        >
          <span class="workspace-code-completion-label">${escapeHtml(item.label || "")}</span>
          ${item.detail ? `<span class="workspace-code-completion-detail">${escapeHtml(item.detail)}</span>` : ""}
        </button>
      `;
    })
    .join("");

  completions.hidden = false;
  completions.innerHTML = itemMarkup;

  const style = window.getComputedStyle(editor);
  const lineHeight = Number.parseFloat(style.lineHeight) || Number.parseFloat(style.fontSize) * 1.72 || 20;
  const paddingLeft = Number.parseFloat(style.paddingLeft) || 0;
  const paddingTop = Number.parseFloat(style.paddingTop) || 0;
  const paddingBottom = Number.parseFloat(style.paddingBottom) || 0;
  const charWidth = Math.max(6, Number.parseFloat(style.fontSize) * 0.62 || 7.5);
  const rect = pane.getBoundingClientRect();
  const beforeCursor = String(editor.value || "").slice(0, editor.selectionStart || 0);
  const lineIndex = beforeCursor.split("\n").length - 1;
  const columnIndex = beforeCursor.length - beforeCursor.lastIndexOf("\n") - 1;
  const caretTop = Math.max(0, Math.round(paddingTop + lineIndex * lineHeight - editor.scrollTop));
  const caretLeft = Math.max(0, Math.round(paddingLeft + columnIndex * charWidth - editor.scrollLeft));
  const listWidth = Math.min(320, Math.max(220, Math.round(rect.width * 0.38)));
  const listHeight = Math.min(220, list.length * 34 + 8);
  const fitsBelow = caretTop + listHeight + 16 < rect.height;
  const top = fitsBelow ? caretTop + lineHeight + 8 : Math.max(8, caretTop - listHeight - 8);
  const left = Math.min(Math.max(8, caretLeft), Math.max(8, rect.width - listWidth - 8));

  completions.style.left = `${left}px`;
  completions.style.top = `${top}px`;
  completions.style.width = `${listWidth}px`;
  completions.style.maxHeight = `${Math.max(140, Math.min(220, rect.height - paddingTop - paddingBottom - 16))}px`;
  completions.dataset.activeLabel = activeItem?.label || "";

  completions.querySelectorAll("[data-code-completion-index]").forEach((button) => {
    button.addEventListener("mousedown", (event) => event.preventDefault());
    button.addEventListener("click", () => {
      const index = Number(button.getAttribute("data-code-completion-index") || 0);
      const nextItem = list[index];
      if (nextItem) {
        applyWorkspaceCodeCompletion(nextItem);
      }
    });
  });
}

function hideWorkspaceCodeCompletions() {
  const { completions } = workspaceEditorNodes();
  if (completions) {
    completions.hidden = true;
    completions.innerHTML = "";
  }
  workspaceCompletionState.visible = false;
  workspaceCompletionState.items = [];
  workspaceCompletionState.activeIndex = 0;
  workspaceCompletionState.lastQuery = "";
}

function syncWorkspaceCodeDecorations() {
  const { editor, highlight, gutter } = workspaceEditorNodes();
  if (!editor || !highlight || !gutter || !currentWorkspaceFile) {
    hideWorkspaceCodeCompletions();
    return;
  }

  const text = String(editor.value || "");
  const language = workspaceEditorLanguage();
  highlight.innerHTML = highlightCode(text || " ", language);
  gutter.innerHTML = buildWorkspaceEditorGutter(text);

  const scrollTop = editor.scrollTop || 0;
  const scrollLeft = editor.scrollLeft || 0;
  highlight.style.transform = `translate(${-scrollLeft}px, ${-scrollTop}px)`;
  gutter.style.transform = `translateY(${-scrollTop}px)`;

  if (workspaceCompletionState.visible && workspaceCompletionState.items.length) {
    const tokenInfo = workspaceCompletionToken(text, editor.selectionStart || 0);
    if (tokenInfo.tokenPrefix === workspaceCompletionState.tokenPrefix) {
      renderWorkspaceCodeCompletions(workspaceCompletionState.items, tokenInfo);
    } else {
      hideWorkspaceCodeCompletions();
    }
  }
}

function applyWorkspaceCodeCompletion(item) {
  const { editor } = workspaceEditorNodes();
  if (!editor || !item) return;
  const tokenInfo = workspaceCompletionToken(editor.value, editor.selectionStart || 0);
  const insertText = String(item.insert_text || item.label || "");
  const nextValue =
    editor.value.slice(0, tokenInfo.tokenStart) +
    insertText +
    editor.value.slice(tokenInfo.tokenEnd);
  const nextCursor = tokenInfo.tokenStart + insertText.length;
  editor.value = nextValue;
  editor.setSelectionRange(nextCursor, nextCursor);
  markWorkspaceEditorDirty(nextValue !== String(currentWorkspaceFile?.content || ""));
  syncWorkspaceCodeDecorations();
  scheduleWorkspaceCodeCompletion(true);
}

function scheduleWorkspaceCodeCompletion(immediate = false) {
  window.clearTimeout(workspaceCompletionTimer);
  const { editor } = workspaceEditorNodes();
  if (!editor || !currentWorkspaceFile || !workspaceCodeCanEdit(currentWorkspaceFile)) {
    hideWorkspaceCodeCompletions();
    return;
  }
  if (workspaceEditorComposing) {
    hideWorkspaceCodeCompletions();
    return;
  }

  const tokenInfo = workspaceCompletionToken(editor.value, editor.selectionStart || 0);
  const query = tokenInfo.tokenPrefix.trim();
  const localItems = workspaceLocalCompletionItems(query, editor.value, workspaceEditorLanguage());
  if (localItems.length) {
    renderWorkspaceCodeCompletions(localItems, tokenInfo);
  } else {
    hideWorkspaceCodeCompletions();
  }

  if (query.length < 2) return;

  const requestId = ++workspaceCompletionRequestId;
  workspaceCompletionState.lastQuery = `${currentWorkspaceFile?.path || ""}:${tokenInfo.tokenStart}:${query}`;

  const runRemoteCompletion = async () => {
    try {
      const prefixContext = editor.value.slice(Math.max(0, tokenInfo.tokenStart - 1200), tokenInfo.tokenEnd);
      const suffixContext = editor.value.slice(tokenInfo.tokenEnd, Math.min(editor.value.length, tokenInfo.tokenEnd + 800));
      const response = await hostClient.workspace.completeFile({
        path: currentWorkspaceFile?.path || "",
        language: workspaceEditorLanguage(),
        token_prefix: tokenInfo.tokenPrefix,
        prefix: prefixContext,
        suffix: suffixContext,
        cursor_line: tokenInfo.lineNumber || 1,
        cursor_column: tokenInfo.columnNumber || 1,
      });

      if (!response.ok) return;
      const payload = await response.json();
      if (requestId !== workspaceCompletionRequestId) return;
      const remoteItems = Array.isArray(payload?.data?.items) ? payload.data.items : [];
      const merged = [];
      const seen = new Set();
      [...localItems, ...remoteItems]
        .filter((item) => item && (item.label || item.insert_text))
        .forEach((item) => {
          const key = String(item.insert_text || item.label || "").toLowerCase();
          if (!key || seen.has(key)) return;
          seen.add(key);
          merged.push({
            label: String(item.label || item.insert_text || ""),
            insert_text: String(item.insert_text || item.label || ""),
            detail: String(item.detail || ""),
            source: String(item.source || "syntax"),
          });
        });
      if (merged.length) {
        renderWorkspaceCodeCompletions(merged, tokenInfo);
      }
    } catch (_error) {
      // Keep local completions if remote completion fails.
    }
  };

  if (immediate) {
    runRemoteCompletion();
    return;
  }

  workspaceCompletionTimer = window.setTimeout(runRemoteCompletion, 280);
}

function syncWorkspaceCodeSaveButton() {
  if (!workspaceCodeSaveButton) return;
  const canEdit = workspaceCodeCanEdit();
  const renderedMarkdown = isMarkdownWorkspaceFile(currentWorkspaceFile) && workspaceCodeRenderMode === "rendered";
  workspaceCodeSaveButton.hidden = !canEdit || renderedMarkdown;
  workspaceCodeSaveButton.disabled = !workspaceEditorDirty;
  workspaceCodeSaveButton.classList.toggle("is-dirty", workspaceEditorDirty);
  workspaceCodeSaveButton.textContent = currentLanguage === "zh"
    ? (workspaceEditorDirty ? "保存" : "已保存")
    : (workspaceEditorDirty ? "Save" : "Saved");
}

function markWorkspaceEditorDirty(nextDirty) {
  workspaceEditorDirty = Boolean(nextDirty);
  syncWorkspaceCodeSaveButton();
  renderWorkspaceTabs();
}

function updateWorkspaceCodeView() {
  if (!workspaceCodeContent) return;
  renderWorkspaceTabs();
  renderWorkspaceBreadcrumbs(currentWorkspaceFile);
  if (!currentWorkspaceFile) {
    disposeWorkspaceMonaco();
    workspaceCodeContent.className = "workspace-code-placeholder";
    workspaceCodeContent.textContent = "Double-click a file to preview its contents.";
    markWorkspaceEditorDirty(false);
    syncWorkspaceCodeRenderToggle();
    syncWorkspaceCodeToolbar();
    return;
  }

  const previewKind = workspaceFilePreviewKind(currentWorkspaceFile);
  const rawUrl = workspaceRawFileUrl(currentWorkspaceFile);
  const isMarkdown = isMarkdownWorkspaceFile(currentWorkspaceFile);
  const fileContent = workspaceFileDisplayContent(currentWorkspaceFile);
  if (previewKind === "image") {
    disposeWorkspaceMonaco();
    workspaceCodeContent.className = "workspace-code-media workspace-code-image-shell";
    workspaceCodeContent.innerHTML = rawUrl
      ? `<img class="workspace-code-image" src="${escapeHtml(rawUrl)}" alt="${escapeHtml(currentWorkspaceFile.name || currentWorkspaceFile.path || "image")}" />`
      : `<div class="workspace-code-unsupported">${escapeHtml(currentLanguage === "zh" ? "图片预览不可用" : "Image preview unavailable")}</div>`;
  } else if (previewKind === "pdf") {
    disposeWorkspaceMonaco();
    workspaceCodeContent.className = "workspace-code-media workspace-code-document-shell";
    workspaceCodeContent.innerHTML = rawUrl
      ? `<iframe class="workspace-code-document" src="${escapeHtml(rawUrl)}" title="${escapeHtml(currentWorkspaceFile.name || "PDF")}"></iframe>`
      : `<div class="workspace-code-unsupported">${escapeHtml(currentLanguage === "zh" ? "PDF 预览不可用" : "PDF preview unavailable")}</div>`;
  } else if (previewKind === "audio") {
    disposeWorkspaceMonaco();
    workspaceCodeContent.className = "workspace-code-media workspace-code-av-shell";
    workspaceCodeContent.innerHTML = rawUrl
      ? `<audio class="workspace-code-audio" controls preload="metadata" src="${escapeHtml(rawUrl)}"></audio>`
      : `<div class="workspace-code-unsupported">${escapeHtml(currentLanguage === "zh" ? "音频预览不可用" : "Audio preview unavailable")}</div>`;
  } else if (previewKind === "video") {
    disposeWorkspaceMonaco();
    workspaceCodeContent.className = "workspace-code-media workspace-code-av-shell";
    workspaceCodeContent.innerHTML = rawUrl
      ? `<video class="workspace-code-video" controls preload="metadata" src="${escapeHtml(rawUrl)}"></video>`
      : `<div class="workspace-code-unsupported">${escapeHtml(currentLanguage === "zh" ? "视频预览不可用" : "Video preview unavailable")}</div>`;
  } else if (previewKind === "unsupported") {
    disposeWorkspaceMonaco();
    workspaceCodeContent.className = "workspace-code-unsupported";
    workspaceCodeContent.innerHTML = `
      <div class="workspace-code-unsupported-title">${escapeHtml(currentLanguage === "zh" ? "该文件类型暂不支持预览" : "This file type is not supported for preview")}</div>
      <div class="workspace-code-unsupported-meta">${escapeHtml(currentWorkspaceFile.mime_type || "")}</div>
    `;
  } else if (isMarkdown && workspaceCodeRenderMode === "rendered") {
    disposeWorkspaceMonaco();
    workspaceCodeContent.className = "workspace-code-markdown markdown-body";
    workspaceCodeContent.innerHTML = renderMarkdown(fileContent || "");
    markWorkspaceEditorDirty(fileContent !== String(currentWorkspaceFile.content || ""));
  } else {
    workspaceCodeContent.className = "workspace-code-editor-shell workspace-code-editor-shell-monaco";
    workspaceCodeContent.innerHTML = `
      <div class="workspace-monaco-host" id="workspace-monaco-host"></div>
    `;
    markWorkspaceEditorDirty(fileContent !== String(currentWorkspaceFile.content || ""));
    const monacoHost = document.getElementById("workspace-monaco-host");
    mountWorkspaceMonaco(monacoHost, { ...currentWorkspaceFile, content: fileContent }).catch((error) => {
      console.error(error);
      workspaceCodeContent.className = "workspace-code-unsupported";
      workspaceCodeContent.innerHTML = `
        <div class="workspace-code-unsupported-title">${escapeHtml(currentLanguage === "zh" ? "加载代码编辑器失败" : "Failed to load the code editor")}</div>
        <div class="workspace-code-unsupported-meta">${escapeHtml(error?.message || "")}</div>
      `;
    });
  }
  syncWorkspaceCodeRenderToggle();
  syncWorkspaceCodeSaveButton();
  syncWorkspaceCodeToolbar();
}

function panelElement(panelId) {
  return document.querySelector(`[data-panel-id="${panelId}"]`);
}

function panelMinWidth(panelId) {
  return { sidebar: 220, chat: 320, research: 320, tree: 300, code: 560 }[panelId] || 220;
}

function panelCurrentWidth(panelId) {
  const width = Number(dockLayout.widths[panelId] || 0);
  return width > 0 ? width : DEFAULT_DOCK_LAYOUT.widths[panelId] || 280;
}

function panelMaxWidth(panelId) {
  const workspaceWidth = Math.max(dockWorkspace?.clientWidth || 0, window.innerWidth || 0, 1280);
  const chatFloor = panelMinWidth("chat");
  const sharedUpperBound = Math.max(640, workspaceWidth - chatFloor - 24);
  return {
    sidebar: 420,
    research: 460,
    tree: Math.min(420, sharedUpperBound),
    code: Math.min(1400, Math.max(860, sharedUpperBound)),
  }[panelId] || sharedUpperBound;
}

function normalizeDockLayout() {
  dockLayout.order = ["sidebar", "chat", "tree", "code", "research"];
  dockLayout.hidden.chat = false;
  dockLayout.hidden.sidebar = true;
  RIGHT_DOCK_PANEL_IDS.forEach((panelId) => {
    dockLayout.hidden[panelId] = true;
  });
  if (currentMainView !== "browser" && !rightSidebarCollapsed) {
    const targetPanelId = resolvePreferredRightDockPanelId();
    if (RIGHT_DOCK_PANEL_IDS.includes(targetPanelId)) {
      dockLayout.hidden[targetPanelId] = false;
      preferredDockRightSidebarPanelId = targetPanelId;
    }
  }
  if (currentWorkspaceMode !== "research" || !hasResearchStartedForCurrentSession()) {
    dockLayout.hidden.research = true;
  }
}

function applyDockLayout() {
  if (!dockWorkspace) return;
  syncShellLayoutVars();
  normalizeDockLayout();

  const visible = renderedPanelIds();
  const hidden = PANEL_IDS.filter((id) => !visible.includes(id));
  const resizers = Array.from(panelResizers);
  const resizerAssignments = visible.slice(0, -1);
  const resizerByPanelId = new Map();
  const workspaceWidth = dockWorkspace.clientWidth || 0;
  const panelGap = Number.parseFloat(window.getComputedStyle(dockWorkspace).columnGap || window.getComputedStyle(dockWorkspace).gap || "0") || 0;
  const totalItemCount = visible.length + Math.max(0, visible.length - 1);
  const gapTotalWidth = Math.max(0, totalItemCount - 1) * panelGap;
  const resizerTotalWidth = Math.max(0, visible.length - 1) * 12;
  let nonChatWidthBudget = Math.max(0, workspaceWidth - gapTotalWidth - resizerTotalWidth - panelMinWidth("chat"));
  const nonChatVisible = visible.filter((panelId) => panelId !== "chat");
  const plannedWidths = {};

  nonChatVisible.forEach((panelId) => {
    plannedWidths[panelId] = clamp(panelCurrentWidth(panelId), panelMinWidth(panelId), panelMaxWidth(panelId));
  });

  const currentTotalNonChatWidth = nonChatVisible.reduce((sum, panelId) => sum + (plannedWidths[panelId] || 0), 0);
  if (currentTotalNonChatWidth > nonChatWidthBudget && nonChatVisible.length) {
    const shrinkOrder = ["code", "research", "tree", "sidebar"].filter((panelId) => nonChatVisible.includes(panelId));
    let overflow = currentTotalNonChatWidth - nonChatWidthBudget;
    shrinkOrder.forEach((panelId) => {
      if (overflow <= 0) return;
      const minWidth = panelMinWidth(panelId);
      const currentWidth = plannedWidths[panelId] || minWidth;
      const shrinkable = Math.max(0, currentWidth - minWidth);
      const shrinkBy = Math.min(shrinkable, overflow);
      plannedWidths[panelId] = currentWidth - shrinkBy;
      overflow -= shrinkBy;
    });
  }

  PANEL_IDS.forEach((panelId) => {
    const panel = panelElement(panelId);
    if (!panel) return;
    const isHidden = Boolean(dockLayout.hidden[panelId]) && panelId !== "chat";
    panel.hidden = isHidden;
    panel.classList.toggle("is-hidden", isHidden);
    if (panelId === "chat") {
      panel.style.width = "";
      panel.style.flex = "1 1 0";
      panel.style.minWidth = `${panelMinWidth(panelId)}px`;
    } else {
      const width = plannedWidths[panelId] || clamp(panelCurrentWidth(panelId), panelMinWidth(panelId), panelMaxWidth(panelId));
      panel.style.flex = "0 0 auto";
      panel.style.width = `${width}px`;
      panel.style.minWidth = `${panelMinWidth(panelId)}px`;
    }
  });

  resizers.forEach((resizer, index) => {
    const panelId = resizerAssignments[index] || "";
    if (panelId) {
      resizer.setAttribute("data-resizer-after", panelId);
      resizer.classList.remove("is-hidden");
      resizerByPanelId.set(panelId, resizer);
    } else {
      resizer.setAttribute("data-resizer-after", "");
      resizer.classList.add("is-hidden");
    }
  });

  const nextOrder = [];
  visible.forEach((panelId) => {
    const panel = panelElement(panelId);
    if (!panel) return;
    nextOrder.push(panel);
    const resizer = resizerByPanelId.get(panelId);
    if (resizer) {
      nextOrder.push(resizer);
    }
  });

  hidden.forEach((panelId) => {
    const panel = panelElement(panelId);
    if (!panel) return;
    nextOrder.push(panel);
  });

  resizers
    .filter((resizer) => resizer.classList.contains("is-hidden"))
    .forEach((resizer) => {
      nextOrder.push(resizer);
    });

  const currentOrder = Array.from(dockWorkspace.children);
  const sameOrder =
    currentOrder.length === nextOrder.length &&
    currentOrder.every((node, index) => node === nextOrder[index]);

  if (!sameOrder) {
    nextOrder.forEach((node) => dockWorkspace.appendChild(node));
  }
  syncLayoutCornerControls();
  syncWorkspaceLauncherVisibility();
  scheduleWorkspaceMonacoLayout();
}

function closePanelMenu() {
  if (panelMenu) {
    panelMenu.hidden = true;
    panelMenu.innerHTML = "";
  }
}

function hasBrowserSidebarTarget() {
  return Boolean(cleanDisplayText(browserState.currentUrl || "", ""));
}

function availableRightSidebarModes() {
  const modes = ["tree"];
  if (isWorkspaceCodeOpen || currentWorkspaceFile || !dockLayout.hidden.code) {
    modes.push("code");
  }
  if (currentWorkspaceMode === "research" && (hasResearchStartedForCurrentSession() || researchDetailOpen)) {
    modes.push("research");
  }
  if (hasBrowserSidebarTarget()) {
    modes.push("browser");
  }
  return [...new Set(modes)];
}

function resolvePreferredRightDockPanelId() {
  const available = availableRightSidebarModes().filter((item) => item !== "browser");
  const preferred = RIGHT_DOCK_PANEL_IDS.includes(preferredDockRightSidebarPanelId)
    ? preferredDockRightSidebarPanelId
    : "tree";
  if (available.includes(preferred)) return preferred;
  return available[0] || "tree";
}

function syncLayoutCornerControls() {
  leftSidebarToggleButton?.classList.toggle("is-active", Boolean(activeActivityPanel));
  rightSidebarToggleButton?.classList.toggle(
    "is-active",
    currentMainView === "browser" || RIGHT_DOCK_PANEL_IDS.some((id) => !dockLayout.hidden[id]),
  );
  if (appShell) {
    appShell.classList.toggle("has-right-sidebar", currentMainView === "browser" || RIGHT_DOCK_PANEL_IDS.some((id) => !dockLayout.hidden[id]));
  }
  if (activityFlyoutResizer) {
    activityFlyoutResizer.hidden = !activeActivityPanel;
  }
  if (browserSplitResizer) {
    browserSplitResizer.hidden = currentMainView !== "browser";
  }
}

function toggleLeftSidebarVisibility() {
  if (activeActivityPanel) {
    setActivityPanel(null, { preserveMainView: preserveMainViewDuringFlyout() });
    return;
  }
  setActivityPanel(preferredLeftActivityPanel || LEFT_ACTIVITY_ORDER[0]);
}

function showRightSidebarMode(mode) {
  const nextMode = String(mode || "").trim();
  if (!nextMode) return;
  if (nextMode === "browser") {
    if (!hasBrowserSidebarTarget()) return;
    rightSidebarCollapsed = false;
    setMainView("browser");
    return;
  }
  if (!RIGHT_DOCK_PANEL_IDS.includes(nextMode)) return;
  preferredDockRightSidebarPanelId = nextMode;
  rightSidebarCollapsed = false;
  if (currentMainView === "browser") {
    setMainView("chat");
    return;
  }
  saveDockLayout();
  applyDockLayout();
  syncLayoutCornerControls();
}

function toggleRightSidebarVisibility() {
  if (currentMainView === "browser") {
    rightSidebarCollapsed = true;
    setMainView("chat");
    return;
  }
  const hasVisibleDockSidebar = RIGHT_DOCK_PANEL_IDS.some((id) => !dockLayout.hidden[id]);
  if (hasVisibleDockSidebar) {
    rightSidebarCollapsed = true;
    saveDockLayout();
    applyDockLayout();
    syncLayoutCornerControls();
    return;
  }
  rightSidebarCollapsed = false;
  showRightSidebarMode(resolvePreferredRightDockPanelId());
}

function reorderPanels(sourceId, targetId) {
  if (!sourceId || !targetId || sourceId === targetId) return;
  const next = dockLayout.order.filter((id) => id !== sourceId);
  const targetIndex = next.indexOf(targetId);
  if (targetIndex < 0) return;
  next.splice(targetIndex, 0, sourceId);
  dockLayout.order = next;
  saveDockLayout();
  applyDockLayout();
}

function stopDockDrag() {
  if (!activeDockDrag) return;
  document.querySelectorAll(".dock-panel").forEach((panel) => panel.classList.remove("is-drag-target"));
  activeDockDrag.handle?.classList.remove("is-dragging");
  activeDockDrag.panel?.classList.remove("is-floating");
  activeDockDrag.panel?.style.removeProperty("--floating-x");
  activeDockDrag.panel?.style.removeProperty("--floating-y");
  document.body.style.cursor = "";
  document.body.style.userSelect = "";
  activeDockDrag = null;
}

function stopResizerDrag() {
  if (!activeResizerDrag) return;
  const { handle, pointerId } = activeResizerDrag;
  if (handle?.releasePointerCapture && pointerId != null) {
    try {
      handle.releasePointerCapture(pointerId);
    } catch (_error) {
      // Ignore pointer capture release failures.
    }
  }
  activeResizerDrag.handle?.classList.remove("is-active");
  appShell?.classList.remove("is-resizing-flyout");
  document.getElementById("workspace-body")?.classList.remove("is-browser-resizing");
  document.body.style.cursor = "";
  document.body.style.userSelect = "";
  activeResizerDrag = null;
}

function dockPanelFromPoint(x, y) {
  const element = document.elementFromPoint(x, y);
  return element?.closest?.("[data-panel-id]")?.getAttribute?.("data-panel-id") || null;
}

function startDockDrag(panelId, handle, pointerId) {
  if (!panelId || !handle) return;
  stopDockDrag();
  const panel = panelElement(panelId);
  activeDockDrag = {
    panelId,
    panel,
    handle,
    pointerId,
    startX: 0,
    startY: 0,
    originX: 0,
    originY: 0,
    dragMode: null,
    moved: false,
    holdReady: false,
    lastInsertIndex: -1,
    currentX: 0,
    currentY: 0,
  };
}

function activateDockReorder(panelId) {
  if (!activeDockDrag || activeDockDrag.panelId !== panelId || activeDockDrag.holdReady) return;
  activeDockDrag.dragMode = "reorder";
  activeDockDrag.holdReady = true;
  suppressNextGripClick = true;
  activeDockDrag.panel?.classList.add("is-floating");
  activeDockDrag.handle?.classList.add("is-dragging");
  document.body.style.userSelect = "none";
  document.body.style.cursor = "grabbing";
}

function dockInsertIndexForX(panelId, clientX) {
  const visible = renderedPanelIds().filter((id) => id !== panelId);
  let insertIndex = visible.length;
  for (let index = 0; index < visible.length; index += 1) {
    const panel = panelElement(visible[index]);
    if (!panel) continue;
    const rect = panel.getBoundingClientRect();
    if (clientX < rect.left + rect.width / 2) {
      insertIndex = index;
      break;
    }
  }
  return { visible, insertIndex };
}

function applyDockReorderForX(panelId, clientX) {
  if (!panelId) return;
  const { visible, insertIndex } = dockInsertIndexForX(panelId, clientX);
  if (activeDockDrag && insertIndex === activeDockDrag.lastInsertIndex) return;
  const next = dockLayout.order.filter((id) => id !== panelId);
  const anchorId = visible[insertIndex] || "";
  if (anchorId) {
    const anchorIndex = next.indexOf(anchorId);
    next.splice(anchorIndex, 0, panelId);
  } else {
    next.push(panelId);
  }
  dockLayout.order = next;
  if (activeDockDrag) {
    activeDockDrag.lastInsertIndex = insertIndex;
  }
  saveDockLayout();
  applyDockLayout();
  if (activeDockDrag) {
    activeDockDrag.panel = panelElement(activeDockDrag.panelId);
  }
}

function highlightDockTarget(clientX) {
  document.querySelectorAll(".dock-panel").forEach((panel) => panel.classList.remove("is-drag-target"));
  if (!activeDockDrag) return;
  const { visible, insertIndex } = dockInsertIndexForX(activeDockDrag.panelId, clientX);
  const targetId = visible[insertIndex] || visible[visible.length - 1] || "";
  if (!targetId) return;
  panelElement(targetId)?.classList.add("is-drag-target");
}

function ensureCodePanelVisible() {
  captureMessageScrollPosition();
  preferredDockRightSidebarPanelId = "code";
  rightSidebarCollapsed = false;
  saveDockLayout();
  applyDockLayout();
  requestAnimationFrame(() => restoreMessageScrollPosition());
}

function hideCodePanel() {
  captureMessageScrollPosition();
  isWorkspaceCodeOpen = false;
  activeWorkspaceFilePath = null;
  currentWorkspaceFile = null;
  workspacePendingReveal = null;
  if (preserveWorkspaceSlotWhenCodeClosed) {
    preferredDockRightSidebarPanelId = "code";
  } else if (preferredDockRightSidebarPanelId === "code") {
    preferredDockRightSidebarPanelId = "tree";
  }
  saveDockLayout();
  applyDockLayout();
  requestAnimationFrame(() => restoreMessageScrollPosition());
  closePanelMenu();
}

function handleGripPointerDown(event, panelId, handle) {
  if (event.type === "mousedown" && Date.now() - lastGripPointerDownAt < 400) {
    return;
  }
  if (event.button !== 0) return;
  if (event.type === "pointerdown") {
    lastGripPointerDownAt = Date.now();
  }
  event.preventDefault();
  event.stopPropagation();
  stopResizerDrag();
  closePanelMenu();
  startDockDrag(panelId, handle, event.pointerId);
  if (activeDockDrag) {
    activeDockDrag.currentX = event.clientX;
    activeDockDrag.currentY = event.clientY;
    activeDockDrag.startX = event.clientX;
    activeDockDrag.startY = event.clientY;
    const rect = activeDockDrag.panel?.getBoundingClientRect();
    activeDockDrag.originX = rect?.x || 0;
    activeDockDrag.originY = rect?.y || 0;
    if (event.type === "pointerdown" && handle.setPointerCapture) {
      try {
        handle.setPointerCapture(event.pointerId);
      } catch (_error) {
        // Ignore unsupported capture failures.
      }
    }
    window.clearTimeout(gripHoldTimer);
    gripHoldTimer = window.setTimeout(() => {
      if (!activeDockDrag || activeDockDrag.panelId !== panelId) return;
      activateDockReorder(panelId);
    }, 160);
  }
}

function onDockPointerMove(event) {
  if (!activeDockDrag) return;
  const deltaX = event.clientX - activeDockDrag.startX;
  const deltaY = event.clientY - activeDockDrag.startY;
  activeDockDrag.currentX = event.clientX;
  activeDockDrag.currentY = event.clientY;
  if (!activeDockDrag.holdReady) {
    if (Math.abs(deltaX) > 4 || Math.abs(deltaY) > 4) {
      window.clearTimeout(gripHoldTimer);
      activeDockDrag.moved = true;
      activateDockReorder(activeDockDrag.panelId);
    }
    if (!activeDockDrag.holdReady) {
      return;
    }
  }

  activeDockDrag.moved = true;
  activeDockDrag.panel?.style.setProperty("--floating-x", `${deltaX}px`);
  activeDockDrag.panel?.style.setProperty("--floating-y", `${deltaY}px`);
  applyDockReorderForX(activeDockDrag.panelId, event.clientX);
  highlightDockTarget(event.clientX);
  activeDockDrag.panel?.classList.add("is-floating");
  activeDockDrag.panel?.style.setProperty("--floating-x", `${deltaX}px`);
  activeDockDrag.panel?.style.setProperty("--floating-y", `${deltaY}px`);
}

function onDockPointerUp(event) {
  if (!activeDockDrag) return;
  window.clearTimeout(gripHoldTimer);
  const holdReady = activeDockDrag.holdReady;
  const moved = activeDockDrag.moved;
  const panelId = activeDockDrag.panelId;
  const handle = activeDockDrag.handle;
  const finalX = event?.clientX ?? activeDockDrag.currentX;
  if ((holdReady || moved) && Number.isFinite(finalX)) {
    applyDockReorderForX(panelId, finalX);
  }
  if (handle?.releasePointerCapture && activeDockDrag.pointerId != null) {
    try {
      handle.releasePointerCapture(activeDockDrag.pointerId);
    } catch (_error) {
      // Ignore unsupported capture failures.
    }
  }
  stopDockDrag();
  if (holdReady || moved) {
    window.setTimeout(() => {
      suppressNextGripClick = false;
    }, 0);
  }
}

function handleResizerPointerDown(event, handle) {
  if (event.button !== 0) return;
  stopDockDrag();
  const kind = handle?.dataset?.layoutResizerKind || "dock";
  if (kind === "flyout") {
    if (!activeActivityPanel || !activityFlyout) return;
    activeResizerDrag = {
      kind,
      handle,
      pointerId: event.pointerId ?? null,
      startX: event.clientX,
    };
    handle.classList.add("is-active");
    appShell?.classList.add("is-resizing-flyout");
  } else if (kind === "browser") {
    if (currentMainView !== "browser" || !browserWorkspace) return;
    activeResizerDrag = {
      kind,
      handle,
      pointerId: event.pointerId ?? null,
      startX: event.clientX,
    };
    handle.classList.add("is-active");
    document.getElementById("workspace-body")?.classList.add("is-browser-resizing");
  } else {
    const afterId = handle?.getAttribute("data-resizer-after") || "";
    const visible = renderedPanelIds();
    const leftIndex = visible.indexOf(afterId);
    const leftPanelId = leftIndex >= 0 ? visible[leftIndex] : "";
    const rightPanelId = leftIndex >= 0 ? visible[leftIndex + 1] || "" : "";
    if (!leftPanelId || !rightPanelId) return;
    activeResizerDrag = {
      kind,
      handle,
      pointerId: event.pointerId ?? null,
      afterId,
      leftPanelId,
      rightPanelId,
      startX: event.clientX,
    };
    handle.classList.add("is-active");
  }
  if (handle?.setPointerCapture && event.pointerId != null) {
    try {
      handle.setPointerCapture(event.pointerId);
    } catch (_error) {
      // Ignore pointer capture failures.
    }
  }
  document.body.style.cursor = "col-resize";
  document.body.style.userSelect = "none";
}

function onResizerPointerMove(event) {
  if (!activeResizerDrag) return;
  if ("buttons" in event && (event.buttons & 1) !== 1) {
    stopResizerDrag();
    return;
  }
  const deltaX = event.clientX - activeResizerDrag.startX;
  if (Math.abs(deltaX) < 1) return;

  if (activeResizerDrag.kind === "flyout") {
    dockLayout.widths.flyout = clamp(
      Number(dockLayout.widths.flyout || DEFAULT_DOCK_LAYOUT.widths.flyout) + deltaX,
      MIN_ACTIVITY_FLYOUT_WIDTH,
      MAX_ACTIVITY_FLYOUT_WIDTH,
    );
    syncShellLayoutVars();
  } else if (activeResizerDrag.kind === "browser") {
    dockLayout.widths.browser = clamp(
      Number(dockLayout.widths.browser || DEFAULT_DOCK_LAYOUT.widths.browser) - deltaX,
      360,
      760,
    );
    syncShellLayoutVars();
  } else {
    const { leftPanelId, rightPanelId } = activeResizerDrag;
    if (leftPanelId !== "chat") {
      dockLayout.widths[leftPanelId] = clamp(
        panelCurrentWidth(leftPanelId) + deltaX,
        panelMinWidth(leftPanelId),
        panelMaxWidth(leftPanelId),
      );
    }
    if (rightPanelId !== "chat") {
      dockLayout.widths[rightPanelId] = clamp(
        panelCurrentWidth(rightPanelId) - deltaX,
        panelMinWidth(rightPanelId),
        panelMaxWidth(rightPanelId),
      );
    }
  }
  activeResizerDrag.startX = event.clientX;
  saveDockLayout();
  applyDockLayout();
}

function applyWorkspaceMode(mode) {
  captureMessageScrollPosition();
  currentWorkspaceMode = mode === "research" ? "research" : "chat";
  try {
    localStorage.setItem("tokitai-workspace-mode", currentWorkspaceMode);
  } catch (_error) {
    // Ignore storage failures.
  }
  if (appShell) {
    appShell.setAttribute("data-mode", currentWorkspaceMode);
  }
  if (researchSection) {
    researchSection.classList.toggle("is-collapsed", currentWorkspaceMode !== "research");
  }
  if (currentWorkspaceMode !== "research") {
    researchDetailOpen = false;
    if (preferredDockRightSidebarPanelId === "research") {
      preferredDockRightSidebarPanelId = isWorkspaceCodeOpen ? "code" : "tree";
    }
  }
  modeButtons.forEach((button) => {
    const active = (button.dataset.mode || "chat") === currentWorkspaceMode;
    button.classList.toggle("is-active", active);
    button.setAttribute("aria-selected", active ? "true" : "false");
  });
  if (messageInput) {
    messageInput.placeholder = currentWorkspaceMode === "research"
      ? (currentLanguage === "zh"
          ? "Agent 默认走轻量实现。输入 /spec 可强制进入研究流程。按 Enter 发送。"
          : "Agent defaults to lightweight implementation. Start with /spec to force a research workflow. Press Enter to send.")
      : t("composerPlaceholder");
  }
  if (composerStop) {
    composerStop.textContent = currentWorkspaceMode === "research"
      ? (currentLanguage === "zh" ? "停止研究" : "Stop research")
      : "Stop";
  }
  if (activityLabel) {
    if (currentWorkspaceMode === "research" && activityLabel === t("activityReviewing")) {
      setActivity(currentLanguage === "zh" ? "研究中" : "Researching");
    } else if (currentWorkspaceMode === "chat" && (
      activityLabel === "研究中" || activityLabel === "Researching"
    )) {
      setActivity(t("activityReviewing"));
    }
  }
  saveDockLayout();
  applyDockLayout();
  syncAgentPreludeBackground(bootstrapData?.messages || []);
  renderResearchFloatingBoard(bootstrapData?.research || null);
  requestAnimationFrame(() => restoreMessageScrollPosition());
}

function setResearchDetailOpen(open) {
  if (open && !hasResearchStartedForCurrentSession()) {
    return;
  }
  researchDetailOpen = Boolean(open) && currentWorkspaceMode === "research";
  if (researchDetailOpen) {
    researchFloatingDismissed = false;
    preferredDockRightSidebarPanelId = "research";
    rightSidebarCollapsed = false;
  } else if (preferredDockRightSidebarPanelId === "research") {
    preferredDockRightSidebarPanelId = isWorkspaceCodeOpen ? "code" : "tree";
  }
  saveDockLayout();
  applyDockLayout();
  renderResearchFloatingBoard(bootstrapData?.research || null);
}

function setSegmentedValue(element, value) {
  if (!element) return;
  const next = String(value ?? "");
  const buttons = Array.from(element.querySelectorAll(".segment"));
  const matched = buttons.some((button) => String(button.dataset.value || "") === next);
  const resolved = matched ? next : String(buttons[0]?.dataset.value || "");
  element.dataset.value = resolved;
  buttons.forEach((button) => {
    button.classList.toggle("is-active", String(button.dataset.value || "") === resolved);
  });
}

function getSegmentedValue(element, fallback) {
  if (!element) return fallback;
  return element.dataset.value ?? fallback;
}

function normalizeChoice(value, allowed, fallback) {
  const next = String(value ?? "");
  return allowed.includes(next) ? next : fallback;
}

function normalizeText(value) {
  return String(value || "")
    .toLowerCase()
    .replace(/\s+/g, " ")
    .trim();
}

function basename(path) {
  const normalized = String(path || "").replace(/[\\/]+$/, "");
  const parts = normalized.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] || normalized || "Workspace";
}

function clipText(value, maxLength = 48) {
  const clean = sanitizeMessageContent(String(value || ""))
    .replace(/[`#>*_[\]]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  if (!clean) return "";
  if (clean.length <= maxLength) return clean;
  return `${clean.slice(0, Math.max(0, maxLength - 3)).trim()}...`;
}

function looksLikeCorruptedText(value) {
  const text = String(value || "").trim();
  if (!text) return false;
  const countChars = (chars) => [...text].filter((char) => chars.includes(char)).length;
  const replacementCount = countChars(["�"]);
  const questionCount = countChars(["?"]);
  const mojibakePunctuation = countChars(["闂", "閵", "婵", "濞", "閺"]);
  const mojibakeMarkers = countChars(["闂", "濞", "椤", "缂", "娓", "瀵", "閻", "娴", "濮"]);
  if (replacementCount > 0) return true;
  if (questionCount >= 4 && questionCount / text.length > 0.18) return true;
  if (questionCount >= 6 && questionCount / text.length > 0.3) return true;
  return (mojibakePunctuation + mojibakeMarkers) >= Math.max(4, Math.floor(text.length * 0.22));
}

function clipDisplayText(value, maxLength = 48) {
  const clipped = clipText(value, maxLength);
  return looksLikeCorruptedText(clipped) ? "" : clipped;
}

function cleanDisplayText(value, fallback = "") {
  const text = String(value ?? "");
  if (!text.trim()) return "";
  return looksLikeCorruptedText(text) ? fallback : text;
}

function decodeEscapedRuntimeText(value) {
  return String(value || "")
    .replace(/\\"/g, "\"")
    .replace(/\\\\/g, "\\")
    .replace(/\\n/g, "\n")
    .replace(/\\r/g, "")
    .replace(/\\t/g, " ");
}

function prettifyRuntimeBranchNote(value) {
  const raw = cleanDisplayText(value || "").trim();
  if (!raw) return "";
  const decoded = decodeEscapedRuntimeText(raw);
  const readField = (name) => {
    try {
      const parsed = JSON.parse(decoded);
      return cleanDisplayText(parsed?.[name] || "");
    } catch (_error) {
      const token = '"' + String(name || "") + '":"';
      const start = decoded.toLowerCase().indexOf(token.toLowerCase());
      if (start < 0) return "";
      const rest = decoded.slice(start + token.length);
      const end = rest.indexOf("\"");
      return cleanDisplayText(end >= 0 ? rest.slice(0, end) : rest);
    }
  };
  const prefixMatch = decoded.match(/^([^"{]+)/);
  const prefix = cleanDisplayText(prefixMatch ? prefixMatch[1] : "");
  const operation = readField("operation");
  const path = readField("path");
  const suggestion = readField("suggestion");
  const message = readField("message");
  const target = displayFileNameOnly(path);

  if (message || suggestion || operation) {
    const operationLabel = cleanDisplayText(prefix || operation);
    const targetSuffix = target ? (" " + target) : "";
    if (/os error 3/i.test(message || decoded)) {
      return currentLanguage === "zh"
        ? (operationLabel || "路径检查") + ": 目标路径" + targetSuffix + " 暂不可用，我会先修正路径再继续。"
        : (operationLabel || "Path check") + ": the target path" + targetSuffix + " is not ready yet, so I am correcting it before continuing.";
    }
    if (/unsupported|not supported/i.test(message || decoded)) {
      return currentLanguage === "zh"
        ? (operationLabel || "文件编辑") + ": 当前编辑方式不受支持，我会切换到可用方式后继续。"
        : (operationLabel || "File edit") + ": that edit mode was not supported, so I am switching to a valid one and continuing.";
    }
    if (/did not match|not match exactly|source text/i.test(message || decoded)) {
      return currentLanguage === "zh"
        ? (operationLabel || "文件编辑") + ": 原始文本未精确匹配，我会重新定位后再继续编辑。"
        : (operationLabel || "File edit") + ": the source text did not match exactly, so I am re-locating it before editing again.";
    }
    const parts = [
      operationLabel,
      cleanDisplayText(message),
      cleanDisplayText(suggestion),
    ].filter(Boolean);
    if (parts.length) {
      return parts.join(currentLanguage === "zh" ? "： " : ": ");
    }
  }

  return raw;
}

function normalizeBranchNotes(values) {
  return (Array.isArray(values) ? values : [])
    .map((item) => prettifyRuntimeBranchNote(item))
    .filter(Boolean)
    .filter((item, index, list) => list.indexOf(item) === index)
    .slice(-6);
}

function isNarrationMergeCandidate(text) {
  const source = String(text || "");
  if (!source.trim()) return false;
  if (source.length > 600) return false;
  return !(new RegExp("```|^\\s*[-*]\\s+", "m")).test(source);
}

function splitNarrationClauses(text) {
  return String(text || "")
    .replace(/\r\n/g, "\n")
    .split(/\n+/)
    .flatMap((line) => line
      .split(/(?<=[。！？.!?])/)
      .map((item) => item.trim())
      .filter(Boolean))
    .map((raw) => ({ raw, normalized: normalizeNarrationClause(raw) }))
    .filter((item) => item.normalized);
}

function normalizeNarrationClause(text) {
  return sanitizeMessageContent(String(text || ""))
    .toLowerCase()
    .replace(/^#+\s*/g, "")
    .replace(/[`*_#]/g, " ")
    .replace(/^(?:let me|i am|i'm|i will|i'll|next|first|starting|continuing)\b[:\s-]*/g, "")
    .replace(/^(?:正在|先|我来|我会|继续)\s*/g, "")
    .replace(/[:：,\s]+/g, "")
    .trim();
}

function areNarrationClausesSimilar(left, right) {
  if (!left || !right) return false;
  if (left === right) return true;
  if (left.includes(right) || right.includes(left)) return true;
  const shared = [...left].filter((char) => right.includes(char)).length;
  const ratio = shared / Math.max(left.length, right.length, 1);
  return ratio >= 0.8;
}

function joinNarrationClauses(clauses) {
  return clauses.reduce((acc, clause) => {
    const raw = String(clause?.raw || "").trim();
    if (!raw) return acc;
    if (!acc) return raw;
    const joiner = /^#+\s*/.test(raw) ? "\n\n" : " ";
    return `${acc}${joiner}${raw}`;
  }, "");
}

function mergeNarrationClauses(existing, next) {
  if (!isNarrationMergeCandidate(existing) || !isNarrationMergeCandidate(next)) {
    return "";
  }
  const leftClauses = splitNarrationClauses(existing);
  const rightClauses = splitNarrationClauses(next);
  if (!leftClauses.length || !rightClauses.length) return "";
  const maxOverlap = Math.min(leftClauses.length, rightClauses.length, 3);
  let overlap = 0;
  for (let size = maxOverlap; size >= 1; size -= 1) {
    let matched = true;
    for (let index = 0; index < size; index += 1) {
      if (!areNarrationClausesSimilar(
        leftClauses[leftClauses.length - size + index]?.normalized,
        rightClauses[index]?.normalized,
      )) {
        matched = false;
        break;
      }
    }
    if (matched) {
      overlap = size;
      break;
    }
  }
  if (!overlap) return "";
  return joinNarrationClauses([
    ...leftClauses,
    ...rightClauses.slice(overlap),
  ]);
}

function isAssistantProcessPreambleParagraph(text) {
  const raw = String(text || "").trim();
  if (!raw) return false;
  if (/^#{1,6}\s/.test(raw) || /^[-*+]\s/.test(raw) || /^\d+\.\s/.test(raw)) return false;
  const normalized = raw.replace(/\s+/g, " ").trim();
  const leadSignal = /^(?:let me|i am|i'm|i will|i'll|i found|i've found|i located|i'm going to|next,? i|first,? i|现在|接下来|我先|我会|我来|先|首先)/i;
  if (!leadSignal.test(normalized)) return false;
  const processSignal = /\b(?:stream|workspace|tool|command|search|inspect|read|edit|write|build|run|verify|check|scan|file|directory|repo|git)\b/i;
  return processSignal.test(normalized) || /(?:工作区|文件|目录|工具|步骤|验证|检查|编辑|读取|扫描|仓库|代码)/.test(normalized);
}

function isAssistantProcessNarrationParagraph(text) {
  const raw = String(text || "").trim();
  if (!raw) return false;
  if (isAssistantCompletionSummaryText(raw)) return false;
  if (/(?:\u672c\u8f6e|\u8fd9\u8f6e).*(?:\u4e2d\u65ad|\u5931\u8d25|\u505c\u6b62|\u6682\u505c)|(?:stream task panicked|panic|error|failed|interrupted|stopped early)/i.test(raw)) return false;
  if (/^#{1,6}\s/.test(raw) || /^[-*+]\s/.test(raw) || /^\d+\.\s/.test(raw) || /^\|.+\|$/.test(raw) || /^```/.test(raw)) {
    return false;
  }
  if (raw.length > 220) return false;
  if (/^[([]?\s*(?:正在执行|正在查看|正在读取|正在检查|running|inspecting|reading|checking)\s*[:：]/i.test(raw)) {
    return true;
  }
  const normalized = raw.replace(/\s+/g, " ").trim();
  const compact = normalizeText(raw).replace(/\s+/g, "");
  const leadSignal = /^(?:let me|first i|i will|i'm going to|next|now|现在|接下来|我先|我会|我来|继续)/i;
  if (!leadSignal.test(normalized) && !/letmefirstcheck|workspace|工作区/.test(compact)) return false;
  const processSignal = /\b(?:workspace|report|script|result|tool|file|directory|inspect|check|scan|edit|read|write|verify)\b/i;
  return processSignal.test(normalized) || /(?:工作区|文件|目录|工具|结果|脚本|检查|扫描|编辑|读取|验证)/.test(normalized);
}

function isAssistantOperationalStatusParagraph(text) {
  const raw = String(text || "").trim();
  if (!raw) return false;
  if (isAssistantCompletionSummaryText(raw)) return false;
  if (/(?:\u672c\u8f6e|\u8fd9\u8f6e).*(?:\u4e2d\u65ad|\u5931\u8d25|\u505c\u6b62|\u6682\u505c)|(?:stream task panicked|panic|error|failed|interrupted|stopped early)/i.test(raw)) return false;
  if (/^#{1,6}\s/.test(raw) || /^[-*+]\s/.test(raw) || /^\d+\.\s/.test(raw) || /^\|.+\|$/.test(raw) || /^```/.test(raw)) {
    return false;
  }
  if (raw.length > 220) return false;
  if (/(?:\u5df2\u5b8c\u6210|\u5df2\u5199\u5165|\u5df2\u521b\u5efa|\u5df2\u4fee\u6539|\u5199\u5165\u5b8c\u6210|\u521b\u5efa\u5b8c\u6210)|(?:completed|written|created|updated|finished successfully)/i.test(raw)) return false;
  const workspaceSignal = /\b(?:csv|workspace|directory|file|script|report|repo|git)\b/i.test(raw)
    || /(?:工作区|目录|文件|脚本|报告|仓库|Git)/.test(raw);
  const actionSignal = /\b(?:from scratch|create|write|run|generate|inspect|check|scan|edit|read|verify)\b/i.test(raw)
    || /(?:创建|写入|运行|生成|检查|扫描|编辑|读取|验证)/.test(raw);
  return workspaceSignal && actionSignal;
}

function extractAssistantOperationalMoment(text, options = {}) {
  const raw = sanitizeMessageContent(String(text || "")).trim();
  if (!raw) return null;
  if (isAssistantCompletionSummaryText(raw)) return null;
  if (/(?:\u5df2\u5b8c\u6210|\u5df2\u5199\u5165|\u5df2\u521b\u5efa|\u5df2\u4fee\u6539|\u5199\u5165\u5b8c\u6210|\u521b\u5efa\u5b8c\u6210)|(?:completed|written|created|updated|finished successfully)/i.test(raw)) return null;
  if (/(?:\u672c\u8f6e|\u8fd9\u8f6e).*(?:\u4e2d\u65ad|\u5931\u8d25|\u505c\u6b62|\u6682\u505c)|(?:stream task panicked|panic|error|failed|interrupted|stopped early)/i.test(raw)) return null;
  if (raw.length > 160) return null;
  if (!isAssistantOperationalStatusParagraph(raw) && !/^[([]?\s*(?:准备执行|准备查看|准备读取|准备检查|正在执行|正在查看|正在读取|正在检查|running|inspecting|reading|checking)\s*[:：]/i.test(raw)) {
    return null;
  }

  const compact = raw
    .replace(/^[([]+\s*/, "")
    .replace(/\s*[\])]+$/, "")
    .trim();
  const normalized = compact.replace(/\s+/g, " ").trim();
  const match = normalized.match(/^(?:准备执行|准备查看|准备读取|准备检查|正在执行|正在查看|正在读取|正在检查|running|inspecting|reading|checking)\s*[:：]?\s*(.+)$/i);
  const detailRaw = cleanDisplayText(match?.[1] || normalized, "");
  if (!detailRaw) return null;

  const detail = detailRaw
    .replace(/^[`"'“”‘’]+|[`"'“”‘’]+$/g, "")
    .trim();
  if (!detail) return null;
  if (detail.length > 120) return null;
  if (/[。！？!?]\s*$/.test(detail) || /[，；;,.].{20,}$/.test(detail)) return null;
  if (/^(?:我会|我先|我将|让我|接下来|首先|I will|I'll|Let me|Next[, ]|First[, ])/i.test(detail)) return null;
  const targetLike = /(?:[A-Za-z]:\\|\/|[A-Za-z0-9_.-]+\.[A-Za-z0-9]+|\b(?:frontend|src|app|index|styles|package|cargo|git)\b)/i.test(detail)
    || /(?:文件|目录|仓库|工作区|页面|模块|组件|函数|脚本)/.test(detail);
  if (!targetLike) return null;

  const readingSignal = /\b(?:read|reading|inspect|inspecting|check|checking|scan|scanning)\b/i.test(detail)
    || /(?:读取|查看|检查|扫描|检视)/.test(detail);
  const editingSignal = /\b(?:edit|editing|write|writing|apply_patch|patch|create|creating|update|updating)\b/i.test(detail)
    || /(?:编辑|写入|修改|创建|更新)/.test(detail);

  return {
    kind: editingSignal ? "edit" : "activity",
    text: editingSignal
      ? zhLabel("正在编辑", "Editing")
      : readingSignal
        ? zhLabel("正在查看", "Inspecting")
        : zhLabel("正在执行", "Running"),
    detail,
    state: options.state || "run",
    dedupeKey: `moment:assistant-status:${normalizeText(detail)}`,
    timestamp: Number(options.timestamp) || Date.now(),
  };
}

function stripAssistantProcessPreamble(text) {
  const source = String(text || "").replace(/\r\n/g, "\n").trim();
  if (!source) return "";
  const paragraphs = source.split(/\n{2,}/);
  let start = 0;
  while (
    start < paragraphs.length
    && (
      isAssistantProcessPreambleParagraph(paragraphs[start])
      || isAssistantProcessNarrationParagraph(paragraphs[start])
      || isAssistantOperationalStatusParagraph(paragraphs[start])
    )
  ) {
    start += 1;
  }
  return paragraphs.slice(start).join("\n\n").trim();
}

function stripAssistantProcessNarration(text) {
  const source = String(text || "").replace(/\r\n/g, "\n").trim();
  if (!source) return "";
  const paragraphs = source.split(/\n{2,}/);
  if (paragraphs.length <= 1) return source;
  const filtered = paragraphs.filter((paragraph) =>
    !isAssistantProcessNarrationParagraph(paragraph)
    && !isAssistantOperationalStatusParagraph(paragraph),
  );
  return filtered.length > 0 ? filtered.join("\n\n").trim() : source;
}

function normalizeAssistantConversationContent(value, { preserveNarrationFallback = true } = {}) {
  const sanitized = sanitizeMessageContent(String(value || ""));
  if (!sanitized.trim()) return "";
  const withoutRuntime = stripAssistantRuntimeSummaryPreamble(sanitized);
  const withoutPreamble = stripAssistantProcessPreamble(withoutRuntime);
  const withoutNarration = stripAssistantProcessNarration(withoutPreamble);
  if (withoutNarration.trim()) return withoutNarration.trim();
  return preserveNarrationFallback ? withoutPreamble.trim() : "";
}

function normalizedAssistantSubstantiveContent(value) {
  return normalizeAssistantConversationContent(value, { preserveNarrationFallback: false }).trim();
}

function isAssistantCompletionSummaryText(value) {
  const raw = sanitizeMessageContent(String(value || "")).trim();
  if (!raw) return false;
  return /(?:\u5df2\u5b8c\u6210|\u5df2\u5199\u5165|\u5df2\u521b\u5efa|\u5df2\u4fee\u6539|\u5199\u5165\u5b8c\u6210|\u521b\u5efa\u5b8c\u6210|\u5df2\u6210\u529f|\u6210\u529f\u5c06.{0,80}(?:\u5199\u5165|\u521b\u5efa|\u4fee\u6539|\u66f4\u65b0)|(?:\u6587\u4ef6|\u5185\u5bb9).{0,40}\u5df2.{0,20}(?:\u5199\u5165|\u521b\u5efa|\u4fee\u6539|\u66f4\u65b0))|(?:completed|written|created|updated|finished successfully|successfully (?:wrote|created|updated|saved))/i.test(raw);
}

function assistantTextLooksLikeProcessNarration(value) {
  const raw = sanitizeMessageContent(String(value || "")).trim();
  if (!raw) return false;
  if (isAssistantCompletionSummaryText(raw)) return false;
  if (normalizedAssistantSubstantiveContent(raw) && !isAssistantOperationalStatusParagraph(raw)) return false;
  const paragraphs = raw.split(/\n{2,}/).map((item) => item.trim()).filter(Boolean);
  if (!paragraphs.length) return false;
  return paragraphs.every((paragraph) =>
    isAssistantProcessPreambleParagraph(paragraph)
    || isAssistantProcessNarrationParagraph(paragraph)
    || isAssistantOperationalStatusParagraph(paragraph)
    || !normalizeAssistantConversationContent(paragraph, { preserveNarrationFallback: false }).trim(),
  );
}

function looksLikeToolPayloadDump(value) {
  const raw = sanitizeMessageContent(String(value || "")).trim();
  if (!raw) return false;
  const normalized = raw.toLowerCase();

  if (
    /"children"\s*:/.test(normalized)
    && /"kind"\s*:\s*"file"/.test(normalized)
    && /"path"\s*:/.test(normalized)
  ) {
    return true;
  }

  if (
    /\b(?:list_dir|find_files|read_file(?:_range)?|tree_dir|search_files|mkdir|rename_path|apply_patch|search_and_replace(?:_multi)?)\b/.test(normalized)
    && /(?:^|[\s{"])data(?:[\s"]|:)/.test(normalized)
  ) {
    return true;
  }

  try {
    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object") return false;
    const payload = parsed.data && typeof parsed.data === "object" ? parsed.data : parsed;
    if (
      payload
      && typeof payload === "object"
      && (
        Array.isArray(payload.children)
        || Array.isArray(payload.tree?.children)
        || (typeof payload.directory === "string" && (payload.tree || payload.children))
      )
    ) {
      return true;
    }
  } catch (_error) {
    // Non-JSON text can still be classified by the heuristics above.
  }

  return false;
}

function looksLikeDirectoryTreeDump(value) {
  const raw = sanitizeMessageContent(String(value || "")).trim();
  if (!raw) return false;
  const lines = raw.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  if (lines.length < 3) return false;

  const boxDrawingLines = lines.filter((line) => /[鈹溾敂鈹傗攢]/.test(line)).length;
  if (boxDrawingLines >= 2) return true;

  const treeLikeLines = lines.filter((line) =>
    /(?:^|[\\/|])[\w.\- ]+\s*(?:#.*)?$/.test(line)
    && /[\\/|]/.test(line)
  ).length;
  if (treeLikeLines >= 4) return true;

  const pathHeavyLines = lines.filter((line) =>
    (line.match(/[\\/]/g) || []).length >= 2
    || /\b(?:readme|requirements\.txt|package\.json|main\.py|config\.py|src|data|models|utils)\b/i.test(line)
  ).length;
  return pathHeavyLines >= 5;
}

function looksLikeOperationalContentDump(value) {
  const raw = sanitizeMessageContent(String(value || "")).trim();
  if (!raw) return false;
  if (isAssistantCompletionSummaryText(raw)) return false;
  if (looksLikeToolPayloadDump(raw) || looksLikeDirectoryTreeDump(raw)) return true;
  if (/^<(?:tool_call|function=|function\s|function_)/i.test(raw)) return true;

  const normalized = raw.toLowerCase();
  if (
    /\b(?:list_dir|find_files|read_file(?:_range)?|tree_dir|search_files|mkdir|rename_path|apply_patch|search_and_replace(?:_multi)?)\b/.test(normalized)
    && /(?:鍙傛暟|args?|call_id|tool_result|宸插畬鎴恷completed|success|status)/i.test(raw)
  ) {
    return true;
  }

  const lines = raw.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  if (lines.length >= 4) {
    const pathLikeLines = lines.filter((line) =>
      /(?:^|[\s(])(?:[A-Za-z]:[\\/]|\.{0,2}[\\/]|\/)/.test(line)
      || /(?:\.(?:rs|js|ts|tsx|jsx|py|toml|json|md|yaml|yml|txt))\b/i.test(line)
    ).length;
    if (pathLikeLines >= 4) return true;
  }

  return false;
}

function looksLikeStructuredAssistantReport(value) {
  const text = normalizedAssistantSubstantiveContent(value);
  if (!text) return false;
  if (looksLikeOperationalContentDump(text)) return false;
  if (text.length < 120) return false;
  const lines = text.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  const codeyLineCount = lines.filter((line) =>
    /^(?:def |class |function |import |from |const |let |var |pub |fn |use |if __name__ ==|for\s+\w+\s+in\s+|while\s+|return\b)/.test(line)
  ).length;
  const csvLikeCount = lines.slice(0, Math.min(lines.length, 6)).filter((line) =>
    (line.match(/,/g) || []).length >= 3
    && !/[#*`|]/.test(line)
    && !/\s{2,}/.test(line)
  ).length;
  if (codeyLineCount >= 3 || csvLikeCount >= 4) return false;
  if (/(?:^|\n)#{1,6}\s/m.test(text)) return true;
  if (/^\|.+\|$/m.test(text)) return true;
  if (/[*_]{2}.+[*_]{2}/.test(text)) return true;
  const cues = ["analysis", "discussion", "results", "conclusion", "summary", "findings"];
  const normalized = normalizeText(text);
  return cues.filter((cue) => normalized.includes(normalizeText(cue))).length >= 2;
}

function isAssistantFailureSummaryText(value) {
  const text = sanitizeMessageContent(String(value || "")).trim();
  if (!text) return false;
  return /research turn stopped because verification did not pass|verification did not pass|resumable checkpoint|safe checkpoint/i.test(text);
}

function isAssistantVerificationAppendixText(value) {
  const text = sanitizeMessageContent(String(value || "")).trim();
  if (!text) return false;
  return /(?:^|\n)##\s*(?:verification report)\b/i.test(text)
    || /(?:^|\n)###\s*(?:verification target)\b/i.test(text);
}

function assistantPrimaryReplyCore(value) {
  return normalizedAssistantSubstantiveContent(stripAssistantReplyAppendices(value));
}

function isAssistantPrimaryReplyText(value) {
  const core = assistantPrimaryReplyCore(value);
  if (!core) return false;
  if (isAssistantFailureSummaryText(value) || isAssistantVerificationAppendixText(value)) return false;
  if (isAssistantCompletionSummaryText(core)) {
    return true;
  }
  if (assistantTextLooksLikeProcessNarration(core)) return false;
  if (looksLikeOperationalContentDump(core)) return false;
  return looksLikeStructuredAssistantReport(core) || core.length >= 120;
}

function extractStructuredToolResultContent(value) {
  const raw = String(value || "").trim();
  if (!raw) return "";
  if (looksLikeOperationalContentDump(raw)) return "";
  try {
    const parsed = JSON.parse(raw);
    const candidates = [
      parsed?.data?.content,
      parsed?.content,
      parsed?.result?.content,
      parsed?.message,
    ];
    for (const candidate of candidates) {
      const text = cleanDisplayText(String(candidate || ""), "");
      if (
        !looksLikeOperationalContentDump(text)
        && (
        looksLikeStructuredAssistantReport(text)
        && !assistantTextLooksLikeProcessNarration(text)
        && !/\b(?:list_dir|find_files|read_file(?:_range)?|tree_dir|search_files|mkdir|rename_path|apply_patch|search_and_replace(?:_multi)?)\b/i.test(text)
        )
      ) {
        return text;
      }
    }
  } catch (_error) {
    const text = cleanDisplayText(raw, "");
    if (
      !looksLikeOperationalContentDump(text)
      && (
      looksLikeStructuredAssistantReport(text)
      && !assistantTextLooksLikeProcessNarration(text)
      && !/\b(?:list_dir|find_files|read_file(?:_range)?|tree_dir|search_files|mkdir|rename_path|apply_patch|search_and_replace(?:_multi)?)\b/i.test(text)
      )
    ) {
      return text;
    }
  }
  return "";
}

function preferAssistantMessageContent(existing, next) {
  const left = String(existing || "");
  const right = String(next || "");
  if (!left) return right;
  if (!right) return left;

  const leftSubstantive = normalizedAssistantSubstantiveContent(left);
  const rightSubstantive = normalizedAssistantSubstantiveContent(right);
  const leftNarration = assistantTextLooksLikeProcessNarration(left);
  const rightNarration = assistantTextLooksLikeProcessNarration(right);
  const leftPrimary = isAssistantPrimaryReplyText(left);
  const rightPrimary = isAssistantPrimaryReplyText(right);
  const leftFailure = isAssistantFailureSummaryText(left);
  const rightFailure = isAssistantFailureSummaryText(right);
  const leftAppendix = isAssistantVerificationAppendixText(left);
  const rightAppendix = isAssistantVerificationAppendixText(right);
  const leftCore = assistantPrimaryReplyCore(left);
  const rightCore = assistantPrimaryReplyCore(right);

  if ((rightFailure || rightAppendix) && leftPrimary) {
    return left;
  }
  if ((leftFailure || leftAppendix) && rightPrimary) {
    return right;
  }

  if (leftNarration && rightSubstantive) {
    return right;
  }
  if (rightNarration && leftSubstantive) {
    return left;
  }
  if (leftCore && rightCore) {
    if (leftCore === rightCore) {
      return right;
    }
    if (leftCore.includes(rightCore)) {
      return left;
    }
    if (rightCore.includes(leftCore)) {
      return right;
    }
  }
  if (leftSubstantive && rightSubstantive && leftSubstantive !== rightSubstantive) {
    const rightPrefersReplace =
      /(?:^|\n)(?:#{1,6}\s|\|.+\|)/m.test(rightSubstantive)
      && rightSubstantive.length >= Math.max(120, Math.floor(leftSubstantive.length * 0.7));
    if (rightPrefersReplace && !leftSubstantive.includes(rightSubstantive)) {
      return right;
    }
    if (looksLikeStructuredAssistantReport(left) && looksLikeStructuredAssistantReport(right)) {
      return right;
    }
  }
  return combineAssistantSegments(left, right);
}

function cleanDisplayMarkdown(value, fallback = "") {
  const text = sanitizeMessageContent(String(value ?? ""));
  if (!text.trim()) return "";
  return looksLikeCorruptedText(text) ? fallback : text;
}

function cleanDisplayList(values) {
  return (Array.isArray(values) ? values : [])
    .map((item) => cleanDisplayText(item))
    .filter(Boolean);
}

function corruptedTextFallback() {
  return currentLanguage === "zh"
    ? "旧文本因编码损坏已省略。"
    : "Legacy text omitted due to corrupted encoding.";
}

function isLowValueSummaryText(value) {
  const text = sanitizeMessageContent(String(value || "")).trim();
  if (!text || looksLikeCorruptedText(text)) return true;
  const markers = [
    "cannot understand your message",
    "unable to understand your message",
    "corrupted encoding",
    "garbled characters",
    "please resend",
    "unreadable content",
  ];
  return markers.some((marker) => text.includes(marker));
}

function latestConversationSummary(messages, maxLength = 42) {
  const visibleMessages = Array.isArray(messages) ? messages : [];
  const latestMessage = [...visibleMessages]
    .reverse()
    .find((message) =>
      message?.kind === "message"
      && message?.role !== "user"
      && String(message?.content || "").trim()
      && !isLowValueSummaryText(message?.content || ""),
    )
    || [...visibleMessages]
      .reverse()
      .find((message) =>
        message?.kind === "message"
        && String(message?.content || "").trim()
        && !isLowValueSummaryText(message?.content || ""),
      );
  if (!latestMessage) return "";
  const cleaned = cleanDisplayText(sanitizeMessageContent(latestMessage.content || ""));
  return cleaned ? clipDisplayText(cleaned, maxLength) : "";
}

function visibleMessagesSignature(messages) {
  const visible = Array.isArray(messages) ? messages : [];
  return visible.map((message) => {
    const kind = String(message?.kind || "");
    const role = String(message?.role || "");
    const callId = String(message?.call_id || "");
    const content = sanitizeMessageContent(String(message?.content || "")).trim();
    return `${kind}|${role}|${callId}|${content}`;
  }).join("\n");
}

function displayMarkdownText(value, fallback = corruptedTextFallback()) {
  return cleanDisplayMarkdown(value, fallback);
}

function displayPlainText(value, fallback = "") {
  return cleanDisplayText(value, fallback);
}

function escapeHtml(value) {
  return String(value || "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function sanitizeHref(rawHref) {
  const href = String(rawHref || "").trim();
  if (!href) return "#";
  if (/^(https?:\/\/|\/)/i.test(href)) return href;
  return "#";
}

async function openUrlInAppBrowser(rawHref, options = {}) {
  const href = sanitizeHref(rawHref);
  if (!href || href === "#") {
    showToast(t("searchNoUrl"));
    return;
  }
  if (!/^https?:\/\//i.test(href)) {
    showToast(t("searchNoUrl"));
    return;
  }

  const pushHistory = options.pushHistory !== false;
  const fallbackViewUrl = `/api/browser/view?url=${encodeURIComponent(href)}`;
  browserState.currentUrl = href;
  browserState.currentTitle = "";
  browserState.currentViewUrl = fallbackViewUrl;
  if (pushHistory) {
    browserState.history = browserState.history.slice(0, browserState.historyIndex + 1);
    browserState.history.push(browserState.currentUrl);
    browserState.historyIndex = browserState.history.length - 1;
  }
  browserState.loading = true;
  if (browserToolbarAddress) {
    browserToolbarAddress.textContent = href;
  }
  rightSidebarCollapsed = false;
  setMainView("browser");
  applyDockLayout();
  try {
    const response = await hostClient.search.browserOpen(href);
    if (response.ok) {
      const payload = await response.json();
      const data = payload?.data || {};
      browserState.currentUrl = cleanDisplayText(data.url || href);
      browserState.currentTitle = cleanDisplayText(data.title || "");
      browserState.currentViewUrl = cleanDisplayText(
        data.view_url || fallbackViewUrl,
      );
      if (pushHistory) {
        browserState.history[browserState.historyIndex] = browserState.currentUrl;
      }
      await loadInAppBrowserDocument(data, href);
      scheduleBrowserBlankCheck(browserState.currentUrl);
      if (browserToolbarAddress) {
        browserToolbarAddress.textContent = browserState.currentTitle
          ? `${browserState.currentTitle} - ${browserState.currentUrl}`
          : browserState.currentUrl;
      }
      return;
    }
    const errorText = await response.text();
    console.warn(errorText || `browser open metadata failed: ${response.status}`);
  } catch (error) {
    console.warn("browser metadata lookup failed", error);
  }
  try {
    await loadInAppBrowserDocument({}, href);
    scheduleBrowserBlankCheck(browserState.currentUrl);
  } finally {
    browserState.loading = false;
  }
}

function preserveMainViewDuringFlyout() {
  return currentMainView === "git" || currentMainView === "browser";
}

function workspaceAssetUrl(rawPath) {
  const path = String(rawPath || "").trim();
  if (!path) return "#";
  if (/^(https?:\/\/|data:|blob:|\/)/i.test(path)) return path;
  const normalized = path.replace(/\\/g, "/").replace(/^\.\/+/, "");
  return hostClient.workspace.rawFileUrl(normalized);
}

function escapeRegExp(value) {
  return String(value || "").replace(new RegExp("[|\\\\{}()\\[\\]^$+*?.]", "g"), "\\$&");
}

function normalizeCodeLanguage(language) {
  const key = String(language || "").toLowerCase().trim();
  const aliases = {
    rs: "rust",
    js: "javascript",
    jsx: "javascript",
    mjs: "javascript",
    cjs: "javascript",
    ts: "typescript",
    tsx: "typescript",
    py: "python",
    htm: "html",
    scss: "css",
    less: "css",
    md: "markdown",
    yml: "yaml",
    ps1: "shell",
    bash: "shell",
    zsh: "shell",
    sh: "shell",
  };
  const extensionAliases = {
    c: "c",
    h: "c",
    cc: "cpp",
    cpp: "cpp",
    cxx: "cpp",
    hpp: "cpp",
    cs: "csharp",
    css: "css",
    go: "go",
    html: "html",
    htm: "html",
    ini: "ini",
    java: "java",
    js: "javascript",
    jsx: "javascript",
    json: "json",
    md: "markdown",
    markdown: "markdown",
    mjs: "javascript",
    py: "python",
    rs: "rust",
    sh: "shell",
    bash: "shell",
    zsh: "shell",
    ps1: "shell",
    ts: "typescript",
    tsx: "typescript",
    yaml: "yaml",
    yml: "yaml",
  };

  if (aliases[key]) return aliases[key];
  if (extensionAliases[key]) return extensionAliases[key];

  const fileName = key.split(/[\\/]/).pop() || key;
  const extMatch = fileName.match(/\.([a-z0-9]+)$/i);
  if (extMatch) {
    const ext = extMatch[1].toLowerCase();
    if (extensionAliases[ext]) return extensionAliases[ext];
  }

  return key || "text";
}

const CODE_HIGHLIGHT_GROUPS = {
  javascript: {
    keywords: ["async", "await", "break", "case", "catch", "class", "const", "continue", "default", "delete", "do", "else", "export", "extends", "finally", "for", "from", "function", "if", "import", "in", "instanceof", "let", "new", "of", "return", "static", "super", "switch", "this", "throw", "try", "typeof", "var", "void", "while", "yield"],
    builtins: ["Array", "Boolean", "Date", "Error", "JSON", "Map", "Math", "Number", "Object", "Promise", "RegExp", "Set", "String", "console", "document", "false", "Infinity", "NaN", "null", "true", "undefined", "window"],
  },
  typescript: {
    keywords: ["abstract", "as", "async", "await", "break", "case", "catch", "class", "const", "constructor", "continue", "declare", "default", "delete", "do", "else", "enum", "export", "extends", "finally", "for", "from", "function", "get", "if", "implements", "import", "in", "infer", "instanceof", "interface", "is", "keyof", "let", "module", "namespace", "new", "of", "override", "private", "protected", "public", "readonly", "return", "set", "static", "super", "switch", "this", "throw", "try", "type", "typeof", "var", "void", "while"],
    builtins: ["Array", "Boolean", "Date", "Error", "JSON", "Map", "Math", "Number", "Object", "Promise", "Record", "RegExp", "Set", "String", "false", "null", "true", "undefined", "unknown", "never"],
  },
  python: {
    keywords: ["and", "as", "assert", "async", "await", "break", "case", "class", "continue", "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import", "in", "is", "lambda", "match", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while", "with", "yield"],
    builtins: ["False", "None", "True", "dict", "float", "int", "len", "list", "print", "range", "set", "self", "str", "tuple"],
  },
  rust: {
    keywords: ["as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "static", "struct", "super", "trait", "type", "unsafe", "use", "where", "while"],
    builtins: ["Self", "String", "Vec", "bool", "char", "false", "i32", "i64", "None", "Option", "Result", "Some", "str", "true", "u32", "u64"],
  },
  shell: {
    keywords: ["case", "do", "done", "elif", "else", "esac", "export", "fi", "for", "function", "if", "in", "local", "then", "unset", "while"],
    builtins: ["echo", "exit", "pwd", "read", "test", "true", "false"],
  },
  css: {
    keywords: ["@import", "@media", "@supports", "@keyframes", "@font-face", "@layer"],
    builtins: ["auto", "block", "flex", "grid", "inherit", "none", "relative", "absolute", "fixed", "sticky"],
  },
  html: {
    keywords: ["DOCTYPE"],
    builtins: [],
  },
  json: {
    keywords: [],
    builtins: ["false", "null", "true"],
  },
  markdown: {
    keywords: [],
    builtins: [],
  },
  yaml: {
    keywords: [],
    builtins: ["false", "null", "true"],
  },
};

function codeWordRegex(words) {
  if (!Array.isArray(words) || !words.length) return null;
  return new RegExp("^(?:" + words.map((word) => escapeRegExp(word)).join("|") + ")\\b");
}

function wrapCodeToken(value, className) {
  const escaped = escapeHtml(value);
  return className ? `<span class="code-token ${className}">${escaped}</span>` : escaped;
}

function highlightCode(source, language) {
  const text = String(source || "");
  const lang = normalizeCodeLanguage(language);
  const group = CODE_HIGHLIGHT_GROUPS[lang] || CODE_HIGHLIGHT_GROUPS.javascript;
  const keywordRegex = codeWordRegex(group.keywords);
  const builtinRegex = codeWordRegex(group.builtins);
  const patternsByLanguage = {
    javascript: [
      ["comment", /^(?:\/\/[^\n]*|\/\*[\s\S]*?(?:\*\/|$))/],
      ["string", /^(?:'(?:\\.|[^'\\])*'?|"(?:\\.|[^"\\])*"?|`(?:\\.|[^`\\])*`?)/],
      ["number", /^(?:0x[\da-fA-F]+|\d+(?:\.\d+)?(?:e[+-]?\d+)?)/],
      ["decorator", /^@[A-Za-z_$][\w$]*/],
      ["keyword", keywordRegex],
      ["builtin", builtinRegex],
      ["function", /^[A-Za-z_$][\w$]*(?=\s*\()/],
      ["property", /^[A-Za-z_$][\w$]*(?=\s*:)/],
      ["type", /^[A-Z][A-Za-z0-9_$]*/],
      ["variable", /^[$A-Za-z_][\w$]*/],
    ],
    typescript: [
      ["comment", /^(?:\/\/[^\n]*|\/\*[\s\S]*?(?:\*\/|$))/],
      ["string", /^(?:'(?:\\.|[^'\\])*'?|"(?:\\.|[^"\\])*"?|`(?:\\.|[^`\\])*`?)/],
      ["number", /^(?:0x[\da-fA-F]+|\d+(?:\.\d+)?(?:e[+-]?\d+)?)/],
      ["decorator", /^@[A-Za-z_$][\w$]*/],
      ["keyword", keywordRegex],
      ["builtin", builtinRegex],
      ["function", /^[A-Za-z_$][\w$]*(?=\s*(?:<[^>\n]+>\s*)?\()/],
      ["property", /^[A-Za-z_$][\w$]*(?=\s*:)/],
      ["type", /^[A-Z][A-Za-z0-9_$]*/],
      ["variable", /^[$A-Za-z_][\w$]*/],
    ],
    python: [
      ["comment", /^#[^\n]*/],
      ["string", /^(?:'''[\s\S]*?(?:'''|$)|"""[\s\S]*?(?:"""|$)|'(?:\\.|[^'\\])*'?|"(?:\\.|[^"\\])*"?)/],
      ["number", /^(?:0x[\da-fA-F]+|\d+(?:\.\d+)?)/],
      ["decorator", /^@[A-Za-z_][\w.]*/],
      ["keyword", keywordRegex],
      ["builtin", builtinRegex],
      ["function", /^[A-Za-z_][\w]*(?=\s*\()/],
      ["type", /^[A-Z][A-Za-z0-9_]*/],
      ["variable", /^[A-Za-z_][\w]*/],
    ],
    rust: [
      ["comment", /^(?:\/\/[^\n]*|\/\*[\s\S]*?(?:\*\/|$))/],
      ["string", /^(?:b?"(?:\\.|[^"\\])*"?|r#*"(?:[\s\S]*?)"#*|b?'(?:\\.|[^'\\])*'?)/],
      ["number", /^(?:0x[\da-fA-F_]+|\d[\d_]*(?:\.\d[\d_]*)?)/],
      ["keyword", keywordRegex],
      ["builtin", builtinRegex],
      ["function", /^[A-Za-z_][\w]*(?=\s*(?:::<[^>\n]+>)?\s*\()/],
      ["type", /^[A-Z][A-Za-z0-9_]*/],
      ["variable", /^[A-Za-z_][\w]*/],
    ],
    shell: [
      ["comment", /^#[^\n]*/],
      ["string", /^(?:'(?:\\.|[^'\\])*'?|"(?:\\.|[^"\\])*"?)/],
      ["number", /^\d+/],
      ["variable", /^(?:\$\{?[A-Za-z_][\w]*\}?|\$[0-9@*#!?$-])/],
      ["keyword", keywordRegex],
      ["builtin", builtinRegex],
      ["function", /^[A-Za-z_][\w-]*(?=\s*\()/],
    ],
    json: [
      ["property", /^"(?:\\.|[^"\\])*"(?=\s*:)/],
      ["string", /^"(?:\\.|[^"\\])*"/],
      ["number", /^(?:-?\d+(?:\.\d+)?(?:e[+-]?\d+)?)/i],
      ["builtin", builtinRegex],
    ],
    html: [
      ["comment", /^<!--[\s\S]*?(?:-->|$)/],
      ["keyword", /^<!DOCTYPE[^>]*>/i],
      ["tag", /^<\/?[A-Za-z][\w:-]*/],
      ["property", /^[A-Za-z_:][\w:.-]*(?=\=)/],
      ["string", /^(?:'(?:\\.|[^'\\])*'?|"(?:\\.|[^"\\])*"?)/],
      ["number", /^\d+/],
    ],
    css: [
      ["comment", /^(?:\/\*[\s\S]*?(?:\*\/|$))/],
      ["keyword", /^(?:@[A-Za-z-]+)/],
      ["property", /^[A-Za-z-]+(?=\s*:)/],
      ["string", /^(?:'(?:\\.|[^'\\])*'?|"(?:\\.|[^"\\])*"?)/],
      ["number", /^(?:#[\da-fA-F]{3,8}\b|-?\d+(?:\.\d+)?(?:px|rem|em|%|vh|vw|ms|s|deg)?)/],
      ["builtin", builtinRegex],
      ["type", /^[.#]?[A-Za-z_][\w-]*/],
    ],
    markdown: [
      ["keyword", /^(?:#{1,6}(?=\s)|```[^\n]*|[-*+](?=\s)|\d+\.(?=\s)|>\s?)/],
      ["property", /^\[[^\]]+\](?=\([^)]+\))/],
      ["string", /^\([^)]+\)/],
      ["comment", /^<!--[\s\S]*?(?:-->|$)/],
    ],
    yaml: [
      ["comment", /^#[^\n]*/],
      ["property", /^[A-Za-z0-9_.-]+(?=\s*:)/],
      ["string", /^(?:'(?:\\.|[^'\\])*'?|"(?:\\.|[^"\\])*"?)/],
      ["number", /^(?:-?\d+(?:\.\d+)?)/],
      ["builtin", builtinRegex],
    ],
  };
  const patterns = patternsByLanguage[lang] || [];
  let index = 0;
  let output = "";

  while (index < text.length) {
    const fragment = text.slice(index);
    const whitespace = fragment.match(/^\s+/);
    if (whitespace) {
      output += escapeHtml(whitespace[0]);
      index += whitespace[0].length;
      continue;
    }

    let matched = false;
    for (const [className, regex] of patterns) {
      if (!regex) continue;
      const result = fragment.match(regex);
      if (!result || !result[0]) continue;
      output += wrapCodeToken(result[0], className);
      index += result[0].length;
      matched = true;
      break;
    }

    if (!matched) {
      output += escapeHtml(text[index]);
      index += 1;
    }
  }

  return output;
}

function renderHighlightedCodeBlock(source, language, blockClass = "") {
  const lang = normalizeCodeLanguage(language);
  const extraClass = blockClass ? ` ${blockClass}` : "";
  return `
    <pre class="syntax-code-block${extraClass}"><code class="syntax-highlight language-${escapeHtml(lang)}">${highlightCode(source, lang)}</code></pre>
  `;
}

function renderLatex(expression, options = {}) {
  const source = String(expression || "").trim();
  if (!source) return "";
  const displayMode = Boolean(options.displayMode);
  if (!window.katex?.renderToString) {
    const tag = displayMode ? "div" : "span";
    const className = displayMode ? "math-block math-fallback" : "math-inline math-fallback";
    return `<${tag} class="${className}">${escapeHtml(source)}</${tag}>`;
  }
  try {
    return window.katex.renderToString(source, {
      throwOnError: false,
      displayMode,
      strict: "ignore",
      trust: false,
    });
  } catch (_error) {
    const tag = displayMode ? "div" : "span";
    const className = displayMode ? "math-block math-fallback" : "math-inline math-fallback";
    return `<${tag} class="${className}">${escapeHtml(source)}</${tag}>`;
  }
}

function extractMathSegments(text, { block = false } = {}) {
  const source = String(text || "");
  const segments = [];
  let output = "";
  let index = 0;

  const pushSegment = (raw, displayMode) => {
    const token = `@@MATH${segments.length}@@`;
    segments.push({
      token,
      markup: renderLatex(raw, { displayMode }),
    });
    output += token;
  };

  while (index < source.length) {
    if (block && source.startsWith("\\[", index)) {
      const end = source.indexOf("\\]", index + 2);
      if (end > index + 2) {
        pushSegment(source.slice(index + 2, end), true);
        index = end + 2;
        continue;
      }
    }

    if (!block && source.startsWith("\\(", index)) {
      const end = source.indexOf("\\)", index + 2);
      if (end > index + 2) {
        pushSegment(source.slice(index + 2, end), false);
        index = end + 2;
        continue;
      }
    }

    if (block && source.startsWith("$$", index)) {
      const end = source.indexOf("$$", index + 2);
      if (end > index + 2) {
        pushSegment(source.slice(index + 2, end), true);
        index = end + 2;
        continue;
      }
    }

    if (!block && source[index] === "$") {
      const prev = index > 0 ? source[index - 1] : "";
      if (prev !== "\\") {
        let cursor = index + 1;
        let end = -1;
        while (cursor < source.length) {
          if (source[cursor] === "$" && source[cursor - 1] !== "\\") {
            end = cursor;
            break;
          }
          if (source[cursor] === "\n") break;
          cursor += 1;
        }
        if (end > index + 1) {
          pushSegment(source.slice(index + 1, end), false);
          index = end + 1;
          continue;
        }
      }
    }

    output += source[index];
    index += 1;
  }

  return { text: output, segments };
}

function restoreMathSegments(html, segments) {
  let output = String(html || "");
  (segments || []).forEach((segment) => {
    output = output.replace(segment.token, segment.markup);
  });
  return output;
}

function renderInlineMarkdown(text) {
  const extracted = extractMathSegments(text, { block: false });
  let html = escapeHtml(extracted.text);
  html = html.replace(/`([^`]+)`/g, (_match, code) => `<code>${code}</code>`);
  html = html.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
  html = html.replace(/\*([^*]+)\*/g, "<em>$1</em>");
  html = html.replace(/!\[([^\]]*)\]\(([^)]+)\)/g, (_match, alt, src) => {
    const safeSrc = workspaceAssetUrl(src);
    return `<img src="${escapeHtml(safeSrc)}" alt="${escapeHtml(alt)}" loading="lazy" />`;
  });
  html = html.replace(/\[([^\]]+)\]\(([^)]+)\)/g, (_match, label, href) => {
    const safeHref = sanitizeHref(href);
    return `<a href="${escapeHtml(safeHref)}" data-inline-link="${escapeHtml(safeHref)}" target="_blank" rel="noreferrer">${label}</a>`;
  });
  return restoreMathSegments(html, extracted.segments);
}

function renderMarkdown(input) {
  const normalized = sanitizeMessageContent(String(input || "")).replace(/\r\n/g, "\n").trim();
  const extracted = extractMathSegments(normalized, { block: true });
  const source = extracted.text;
  if (!source) return "";

  const lines = source.split("\n");
  const html = [];
  let index = 0;

  const isListLine = (line) => /^(\s*[-*+]\s+|\s*\d+\.\s+)/.test(line);
  const isQuoteLine = (line) => /^\s*>\s?/.test(line);
  const isHeadingLine = (line) => /^(#{1,6})(\s+|\S)/.test(line);
  const isFenceLine = (line) => /^```/.test(line);
  const splitTableRow = (line) =>
    String(line || "")
      .trim()
      .replace(/^\|/, "")
      .replace(/\|$/, "")
      .split("|")
      .map((cell) => cell.trim());
  const isTableDivider = (line) => {
    const cells = splitTableRow(line);
    return cells.length > 1 && cells.every((cell) => /^:?-{3,}:?$/.test(cell));
  };
  const isTableRow = (line) => {
    const trimmed = String(line || "").trim();
    if (!trimmed.includes("|")) return false;
    const cells = splitTableRow(trimmed);
    return cells.length > 1 && cells.some((cell) => cell.length > 0);
  };

  while (index < lines.length) {
    const line = lines[index];

    if (!line.trim()) {
      index += 1;
      continue;
    }

    if (isFenceLine(line)) {
      const language = line.replace(/^```/, "").trim();
      index += 1;
      const codeLines = [];
      while (index < lines.length && !isFenceLine(lines[index])) {
        codeLines.push(lines[index]);
        index += 1;
      }
      if (index < lines.length && isFenceLine(lines[index])) {
        index += 1;
      }
      html.push(renderHighlightedCodeBlock(codeLines.join("\n"), language || "text", "md-code"));
      continue;
    }

    if (/^@@MATH\d+@@$/.test(line.trim())) {
      html.push(`<div class="math-block-wrap">${line.trim()}</div>`);
      index += 1;
      continue;
    }

    if (
      index + 1 < lines.length &&
      isTableRow(line) &&
      isTableDivider(lines[index + 1])
    ) {
      const headerCells = splitTableRow(line);
      index += 2;
      const bodyRows = [];
      while (index < lines.length && lines[index].trim() && isTableRow(lines[index])) {
        bodyRows.push(splitTableRow(lines[index]));
        index += 1;
      }
      html.push(`
        <div class="markdown-table-wrap">
          <table class="markdown-table">
            <thead>
              <tr>${headerCells.map((cell) => `<th>${renderInlineMarkdown(cell)}</th>`).join("")}</tr>
            </thead>
            <tbody>
              ${bodyRows
                .map(
                  (row) =>
                    `<tr>${headerCells
                      .map((_, columnIndex) => `<td>${renderInlineMarkdown(row[columnIndex] || "")}</td>`)
                      .join("")}</tr>`,
                )
                .join("")}
            </tbody>
          </table>
        </div>
      `);
      continue;
    }

    if (isHeadingLine(line)) {
      const [, hashes, content] = line.match(/^(#{1,6})\s*(.*)$/) || [];
      const level = Math.min(6, hashes.length);
      html.push(`<h${level}>${renderInlineMarkdown(content)}</h${level}>`);
      index += 1;
      continue;
    }

    if (isQuoteLine(line)) {
      const quoteLines = [];
      while (index < lines.length && isQuoteLine(lines[index])) {
        quoteLines.push(lines[index].replace(/^\s*>\s?/, ""));
        index += 1;
      }
      html.push(`<blockquote>${renderInlineMarkdown(quoteLines.join("<br>"))}</blockquote>`);
      continue;
    }

    if (isListLine(line)) {
      const ordered = /^\s*\d+\.\s+/.test(line);
      const tag = ordered ? "ol" : "ul";
      const items = [];
      while (index < lines.length && isListLine(lines[index])) {
        items.push(lines[index].replace(/^(\s*[-*+]\s+|\s*\d+\.\s+)/, ""));
        index += 1;
      }
      html.push(
        `<${tag}>${items.map((item) => `<li>${renderInlineMarkdown(item)}</li>`).join("")}</${tag}>`,
      );
      continue;
    }

    const paragraph = [];
    while (
      index < lines.length &&
      lines[index].trim() &&
      !isFenceLine(lines[index]) &&
      !isHeadingLine(lines[index]) &&
      !isQuoteLine(lines[index]) &&
      !isListLine(lines[index])
    ) {
      paragraph.push(lines[index].trim());
      index += 1;
    }
    html.push(`<p>${renderInlineMarkdown(paragraph.join(" "))}</p>`);
  }

  return restoreMathSegments(html.join(""), extracted.segments);
}

const ASSISTANT_SECTION_CUES = new Set([
  "summary",
  "overview",
  "conclusion",
  "conclusions",
  "finding",
  "findings",
  "result",
  "results",
  "validation",
  "verify",
  "verification",
  "test",
  "tests",
  "check",
  "checks",
  "change",
  "changes",
  "impact",
  "impacts",
  "reason",
  "reasons",
  "risk",
  "risks",
  "status",
  "note",
  "notes",
  "plan",
  "output",
  "outputs",
  "deliverable",
  "deliverables",
  "nextstep",
  "nextsteps",
  "followup",
  "followups",
  "whatchanged",
  "why",
  "summary",
  "findings",
  "analysis",
  "discussion",
  "results",
  "conclusion",
  "statusupdate",
  "notesummary",
  "verificationreport",
  "nextaction",
  "nextactions",
  "changedfiles",
  "rationale",
  "impactsummary",
  "riskreview",
  "outputsummary",
]);

function normalizeAssistantSectionCue(value) {
  return String(value || "")
    .toLowerCase()
    .replace(/[\s:_/()[\]{}-]+/g, "");
}

function protectFencedMarkdownBlocks(text) {
  const blocks = [];
  const protectedText = String(text || "").replace(/```[\s\S]*?```/g, (block) => {
    const token = `@@ASSISTANT_FENCE_${blocks.length}@@`;
    blocks.push({ token, block });
    return token;
  });
  return { text: protectedText, blocks };
}

function restoreFencedMarkdownBlocks(text, blocks) {
  let output = String(text || "");
  (blocks || []).forEach((entry) => {
    output = output.replace(entry.token, entry.block);
  });
  return output;
}

function normalizeAssistantStructuralLine(line) {
  const raw = String(line || "");
  if (/^\s*(#{1,6})\s*#+\s+/.test(raw)) {
    return raw.replace(/^\s*(#{1,6})\s*#+\s+/, (_match, hashes) => `${hashes} `);
  }
  if (/^\s*(#{1,6})([^\s#].*)$/.test(raw)) {
    return raw.replace(/^\s*(#{1,6})([^\s#].*)$/, (_match, hashes, rest) => `${hashes} ${rest}`);
  }
  if (/^\s*[閳モ懇妫岃矾閳活亖妫埢绯曟煙]\s+/.test(raw)) {
    return raw.replace(/^\s*[閳モ懇妫岃矾閳活亖妫埢绯曟煙]\s+/, "- ");
  }
  if (/^\s*(\d+)[\)\uff09]\s+/.test(raw)) {
    return raw.replace(/^\s*(\d+)[\)\uff09]\s+/, (_match, index) => `${index}. `);
  }
  return raw;
}

function stripAssistantRuntimeSummaryPreamble(text) {
  let source = String(text || "").replace(/\r\n/g, "\n");
  if (!source.trim()) return "";
  const runtimeLeadPattern = /^\s*(?:runtime summary|verification report|verification target|research turn stopped because verification did not pass)[^\n]*(?:\n+|$)/i;
  let next = source.replace(runtimeLeadPattern, "");
  while (next !== source) {
    source = next;
    next = source.replace(runtimeLeadPattern, "");
  }
  return source.trimStart();
}

function splitResearchStepNarrationLine(line) {
  const raw = String(line || "");
  if (!raw.trim()) return [raw];
  const markers = [
    "Step 1",
    "Step 2",
    "Step 3",
    "Step 4",
    "首先",
    "接下来",
    "然后",
    "最后",
    "我先",
    "我会",
    "现在",
  ];
  let boundary = -1;
  for (const marker of markers) {
    const index = raw.indexOf(marker);
    if (index > 12 && (boundary === -1 || index < boundary)) {
      boundary = index;
    }
  }
  if (boundary === -1) return [raw];
  return [raw.slice(0, boundary).trimEnd(), raw.slice(boundary).trimStart()].filter(Boolean);
}

function repairCollapsedStreamingStructure(text) {
  const source = String(text || "");
  if (!source) return "";

  let repaired = source
    .replace(/([^\n])\s*(#{1,6})(?=\S)/g, "$1\n\n$2 ")
    .replace(/([^\n])\s*(#{1,6})\s+/g, "$1\n\n$2 ");

  repaired = repaired.replace(
    /^(#{1,6}\s*)([^\n#]{1,18}?)(?=(This\b|In\b|For\b|A\b|An\b))/,
    (_match, prefix, title, starter) => `${prefix}${title}\n\n${starter}`,
  );

  return repaired;
}

function matchAssistantSectionLine(line) {
  const trimmed = String(line || "").trim();
  if (!trimmed) return null;

  const numberedSection = trimmed.match(/^\d+\.\s*(.{1,24})$/);
  if (numberedSection) {
    return {
      title: numberedSection[1].trim(),
      body: "",
    };
  }

  const inlineSection = trimmed.match(/^([^:\n]{1,18})[:：]\s+(.+)$/);
  if (inlineSection) {
    const title = inlineSection[1].trim();
    const body = inlineSection[2].trim();
    const normalizedTitle = normalizeAssistantSectionCue(title);
    if (ASSISTANT_SECTION_CUES.has(normalizedTitle) && body.length >= 22) {
      return {
        title,
        body,
      };
    }
  }

  const standaloneTitle = trimmed.replace(/[:：]\s*$/, "");
  const normalizedStandalone = normalizeAssistantSectionCue(standaloneTitle);
  if (standaloneTitle.length <= 24 && ASSISTANT_SECTION_CUES.has(normalizedStandalone)) {
    return {
      title: standaloneTitle,
      body: "",
    };
  }

  return null;
}

function parseAssistantKeyValueLine(line) {
  const trimmed = String(line || "").trim();
  if (!trimmed) return null;
  const match = trimmed.match(/^([^:閿涙瓡n]{1,20})[閿?]\s+(.+)$/);
  if (!match) return null;
  const label = match[1].trim();
  const value = match[2].trim();
  if (!label || !value) return null;
  const normalizedLabel = normalizeAssistantSectionCue(label);
  const labelWordCount = label.split(/\s+/).filter(Boolean).length;
  if (label.length > 18 && labelWordCount > 4) return null;
  return {
    label,
    value,
    normalizedLabel,
  };
}

function stripAssistantReplyAppendices(input) {
  const source = String(input || "").replace(/\r\n/g, "\n").trim();
  if (!source) return "";
  const inlineFailureMatch = /research turn stopped because verification did not pass/i.exec(source);
  if (inlineFailureMatch && typeof inlineFailureMatch.index === "number" && inlineFailureMatch.index > 0) {
    const head = source.slice(0, inlineFailureMatch.index).trim();
    if (normalizedAssistantSubstantiveContent(head)) {
      return head;
    }
  }
  const markers = [
    /##\s*verification report\b/i,
    /(?:^|\n)validation (?:evidence )?summary[閿?]/i,
    /(?:^|\n)the output above satisfies all constraints/i,
    /research turn stopped because verification did not pass/i,
    /verification did not pass/i,
    /resumable checkpoint/i,
    /safe checkpoint/i,
  ];
  let cutIndex = -1;
  markers.forEach((pattern) => {
    const match = pattern.exec(source);
    if (!match || typeof match.index !== "number" || match.index <= 0) return;
    if (cutIndex === -1 || match.index < cutIndex) {
      cutIndex = match.index;
    }
  });
  if (cutIndex <= 0) return source;
  const head = source.slice(0, cutIndex).trim();
  return normalizedAssistantSubstantiveContent(head) ? head : source;
}

function dedupeAssistantDisplayParagraphs(input) {
  const paragraphs = String(input || "")
    .split(/\n{2,}/)
    .map((item) => item.trim())
    .filter(Boolean);
  if (paragraphs.length <= 1) return String(input || "").trim();
  const seen = new Set();
  const kept = [];
  paragraphs.forEach((paragraph) => {
    const normalized = normalizeText(paragraph).replace(/\s+/g, "");
    const isLongDuplicateCandidate = normalized.length >= 80;
    if (isLongDuplicateCandidate && seen.has(normalized)) {
      return;
    }
    if (isLongDuplicateCandidate) {
      seen.add(normalized);
    }
    kept.push(paragraph);
  });
  return kept.join("\n\n").trim();
}

function structureAssistantDisplayText(input) {
  const source = dedupeAssistantDisplayParagraphs(
    stripAssistantReplyAppendices(
      normalizeAssistantConversationContent(String(input || "")),
    ),
  )
    .replace(/\r\n/g, "\n")
    .replace(/\s+---+\s+/g, "\n\n")
    .replace(/([閵嗗偊绱掗敍?!?])\s*(#{1,6})(?=\S)/g, "$1\n\n$2 ")
    .replace(/([閵嗗偊绱掗敍?!?])\s*(#{1,6})\s+/g, "$1\n\n$2 ")
    .trim();
  if (!source) return "";

  const protectedBlocks = protectFencedMarkdownBlocks(source);
  const lines = protectedBlocks.text.split("\n");
  const output = [];
  const keyValueBuffer = [];

  const flushKeyValueBuffer = () => {
    if (!keyValueBuffer.length) return;
    if (keyValueBuffer.length === 1) {
      const entry = keyValueBuffer[0];
      if (ASSISTANT_SECTION_CUES.has(entry.normalizedLabel) && entry.value.length > 30) {
        output.push(`### ${entry.label}`);
        output.push(entry.value);
      } else {
        output.push(`- **${entry.label}**: ${entry.value}`);
      }
    } else {
      keyValueBuffer.forEach((entry) => {
        output.push(`- **${entry.label}**: ${entry.value}`);
      });
    }
    keyValueBuffer.length = 0;
  };

  lines.forEach((line) => {
    splitResearchStepNarrationLine(line).forEach((part) => {
      const normalizedLine = normalizeAssistantStructuralLine(part);
      const trimmed = normalizedLine.trim();

      if (!trimmed) {
        flushKeyValueBuffer();
        output.push("");
        return;
      }

      if (/^@@ASSISTANT_FENCE_\d+@@$/.test(trimmed)) {
        flushKeyValueBuffer();
        output.push(trimmed);
        return;
      }

      if (/^(#{1,6}\s|>\s?|[-*+]\s+|\d+\.\s+|\|.+\|)/.test(trimmed)) {
        flushKeyValueBuffer();
        output.push(normalizedLine);
        return;
      }

      const section = matchAssistantSectionLine(trimmed);
      if (section) {
        flushKeyValueBuffer();
        output.push(`### ${section.title}`);
        if (section.body) {
          output.push(section.body);
        }
        return;
      }

      const keyValue = parseAssistantKeyValueLine(trimmed);
      if (keyValue) {
        keyValueBuffer.push(keyValue);
        return;
      }

      flushKeyValueBuffer();
      output.push(normalizedLine);
    });
  });

  flushKeyValueBuffer();
  const normalizedOutput = [];
  const blockStarters = /^(#{1,6}\s|[-*+]\s|>\s?|\d+\.\s|```|@@ASSISTANT_FENCE_\d+@@|\|.+\|)/;
  const sentenceBoundary = /[.!?。！？]$/;
  output.forEach((line, index) => {
    const current = String(line || "");
    const trimmed = current.trim();
    const previous = normalizedOutput.length ? String(normalizedOutput[normalizedOutput.length - 1] || "") : "";
    const prevTrimmed = previous.trim();
    const nextRaw = index + 1 < output.length ? String(output[index + 1] || "") : "";
    const nextTrimmed = nextRaw.trim();

    if (!trimmed) {
      if (prevTrimmed) normalizedOutput.push("");
      return;
    }

    const looksDenseParagraph =
      trimmed.length > 90 &&
      !blockStarters.test(trimmed) &&
      !blockStarters.test(prevTrimmed) &&
      !blockStarters.test(nextTrimmed);

    if (
      normalizedOutput.length &&
      prevTrimmed &&
      looksDenseParagraph &&
      sentenceBoundary.test(prevTrimmed)
    ) {
      normalizedOutput.push("");
    }

    normalizedOutput.push(current);
  });
  return restoreFencedMarkdownBlocks(
    normalizedOutput.join("\n").replace(/\n{3,}/g, "\n\n").trim(),
    protectedBlocks.blocks,
  );
}

function renderRuntimeSectionCard(title, countLabel, bodyMarkup, options = {}) {
  if (!bodyMarkup) return "";
  const shouldOpen = options.open === true;
  const tone = cleanDisplayText(options.tone || "", "");
  const extraClass = cleanDisplayText(options.className || "", "");
  return `
    <details class="codex-runtime-card ${escapeHtml(extraClass)}${tone ? ` is-${escapeHtml(tone)}` : ""}"${shouldOpen ? " open" : ""}>
      <summary class="codex-runtime-card-summary">
        <span class="codex-runtime-card-title">${escapeHtml(title)}</span>
        <span class="codex-runtime-card-count">${escapeHtml(countLabel || "")}</span>
      </summary>
      <div class="codex-runtime-card-body">
        ${bodyMarkup}
      </div>
    </details>
  `;
}

function summarizeOperationTools(tools) {
  const list = Array.isArray(tools) ? tools : [];
  if (!list.length) return null;
  const names = list.map((tool) => String(tool?.name || "").trim().toLowerCase());
  const hasWorkspaceScan = names.some((name) => ["list_dir", "find_files", "search_files", "tree_dir", "read_file", "read_file_range"].includes(name));
  const hasEdit = names.some((name) => ["write_file", "apply_patch", "search_and_replace", "search_and_replace_multi", "rename_path", "mkdir"].includes(name));
  const hasGit = names.some((name) => name.startsWith("git_"));
  if (hasWorkspaceScan && !hasEdit && !hasGit) {
    return {
      title: currentLanguage === "zh" ? "我先查看工作区结构和关键文档。" : "I will first inspect the workspace structure and design docs.",
      meta: currentLanguage === "zh" ? "正在查看工作区文件" : "Viewing workspace files",
    };
  }
  if (hasEdit && !hasGit) {
    return {
      title: currentLanguage === "zh" ? "我来修改这段逻辑。" : "I will update this logic.",
      meta: currentLanguage === "zh" ? "正在编辑文件" : "Editing files",
    };
  }
  if (hasGit) {
    return {
      title: currentLanguage === "zh" ? "我先检查改动和仓库状态。" : "I will check the changes and repository state first.",
      meta: currentLanguage === "zh" ? "正在查看 Git 状态" : "Checking Git status",
    };
  }
  return {
    title: currentLanguage === "zh" ? "我继续推进当前步骤。" : "I am continuing the current step.",
    meta: currentLanguage === "zh" ? "正在执行工具" : "Running tools",
  };
}
function renderAssistantRuntimePanel(content, options = {}) {
  if (!content) return "";
  const title = options.title || zhLabel("Execution", "Execution");
  const meta = cleanDisplayText(options.meta || "", "");
  const open = options.open !== false;
  const tone = cleanDisplayText(options.tone || "", "");
  const toneClass = tone ? ` is-${escapeHtml(tone)}` : "";
  const isRunning = tone === "running";
  return `
    <section class="codex-runtime-panel${toneClass}">
      <details class="codex-runtime-panel-shell" data-runtime-panel${open ? " open" : ""}>
        <summary class="codex-runtime-panel-summary" data-runtime-toggle>
          <span class="codex-runtime-panel-title${isRunning ? " is-streaming" : ""}" data-text="${escapeHtml(title)}">${escapeHtml(title)}</span>
          ${meta ? `<span class="codex-runtime-panel-meta">${escapeHtml(meta)}</span>` : ""}
        </summary>
        <div class="codex-runtime-panel-body">
          ${content}
        </div>
      </details>
    </section>
  `;
}

function renderThinkingSummaryLabel(index, isStreaming) {
  const label = currentLanguage === "zh" ? "Thinking..." : "Thinking...";
  return `
    <span class="codex-thinking-summary-shell">
      <span class="codex-thinking-summary-label${isStreaming ? " is-streaming" : ""}">${escapeHtml(label)}</span>
    </span>
  `;
}

function streamAnimationStyle(turn = activeAssistantTurn) {
  const startedAt = Number(turn?.startedAt || 0);
  if (!startedAt) return "";
  const phaseSeconds = ((Date.now() - startedAt) % 2350) / 1000;
  return ` style="--codex-stream-phase:-${phaseSeconds.toFixed(3)}s;"`;
}

function renderOperationSection(title, bodyMarkup) {
  if (!bodyMarkup) return "";
  return `
    <section class="codex-operation-section">
      <div class="codex-operation-section-title">${escapeHtml(title)}</div>
      ${bodyMarkup}
    </section>
  `;
}

function extractAssistantDecisionCard(text) {
  const source = cleanDisplayText(String(text || ""), "");
  if (!source) return { body: "", card: null };
  const normalized = source.replace(/\r\n/g, "\n");
  const lines = normalized.split("\n").map((line) => line.trim());
  const highLevelContext = /\b(?:please confirm|which direction do you prefer|which direction you prefer|if you agree|choose next step|pick one|select one|which option|confirm your choice)\b/i;
  const strategicOptionLine = /^(?:#{2,3}\s*)?(?:direction|approach|strategy|option)\b|^(?:\d+\.\s*)(?:direction|approach|strategy|option)\b/i;
  const lowLevelExecution = /\b(?:file|folder|directory|script|path|create|add|generate|modify|edit|rename|delete|write)\b|\.(?:rs|py|js|ts|tsx|jsx|toml|json|yaml|yml|md)\b/i;
  if (!highLevelContext.test(normalized)) {
    return { body: normalized, card: null };
  }
  const alternativeOptionLines = [];
  const bodyLines = [];

  for (const line of lines) {
    if (!line) {
      continue;
    }

    if (/^please confirm[:?]?$/i.test(line) || /^preparing final message payload/i.test(line)) {
      continue;
    }

    if (strategicOptionLine.test(line) || /^(?:[-*+]\s+|\d+\.\s+)/.test(line)) {
      const cleaned = cleanDisplayText(
        line
          .replace(/^#{2,3}\s*/, "")
          .replace(/^(?:\d+\.\s*|[-*+]\s*)/, "")
          .replace(/[:：]\s*$/, ""),
        "",
      );
      if (
        cleaned &&
        !lowLevelExecution.test(cleaned) &&
        /\b(?:direction|approach|strategy|option)\b/i.test(cleaned) &&
        !/\b(?:summary|comparison|compare)\b/i.test(cleaned)
      ) {
        alternativeOptionLines.push(cleaned);
        continue;
      }
    }

    bodyLines.push(line);
  }

  const options = alternativeOptionLines
    .map((line) => cleanDisplayText(line, ""))
    .filter((line, index, array) => line && array.indexOf(line) === index);

  if (options.length < 2) {
    return { body: normalized, card: null };
  }

  return {
    body: bodyLines.join("\n").trim(),
    card: {
      title: currentLanguage === "zh" ? "选择下一步" : "Choose next step",
      options,
    },
  };
}

function shouldRenderAssistantDecisionCard(turn, cleanedText) {
  const options = Array.isArray(turn?.assistantChoices?.options) ? turn.assistantChoices.options : [];
  if (!options.length) return false;
  const source = cleanDisplayText(String(cleanedText || turn?.text || ""), "");
  if (!source) return false;
  return /\b(?:please confirm|which direction do you prefer|which direction you prefer|if you agree|choose next step|pick one|select one|which option|confirm your choice)\b/i.test(source);
}

function renderAssistantDecisionCard(card) {
  if (!card || !Array.isArray(card.options) || !card.options.length) return "";
  return `
    <div class="codex-decision-card">
      <div class="codex-decision-title">${escapeHtml(card.title || zhLabel("选择下一步", "Choose next step"))}</div>
      <div class="codex-decision-options">
        ${card.options.map((option) => `
          <button class="codex-decision-option" type="button" data-decision-option="${escapeHtml(option)}">${escapeHtml(option)}</button>
        `).join("")}
      </div>
      <div class="codex-decision-custom">
        <textarea rows="2" placeholder="${escapeHtml(zhLabel("输入你的想法", "Enter your idea"))}" data-decision-custom-input></textarea>
        <button class="codex-decision-submit" type="button" data-decision-custom-submit>${escapeHtml(zhLabel("发送", "Send"))}</button>
      </div>
    </div>
  `;
}

function operationEditArtifactMarkup(turn, label) {
  const text = cleanDisplayText(String(label || ""), "");
  if (!turn || !text) return "";
  if (!/(?:正在编辑|editing)/i.test(text)) return "";

  const diffs = Array.isArray(turn?.diffs) ? turn.diffs : [];
  const tools = Array.isArray(turn?.tools) ? turn.tools : [];
  const latestDiff = diffs.length ? diffs[diffs.length - 1] : null;
  const latestTool = [...tools].reverse().find((tool) => {
    const name = String(tool?.name || "").toLowerCase();
    return ["write_file", "apply_patch", "search_and_replace", "search_and_replace_multi", "rename_path", "mkdir"].includes(name);
  }) || null;

  const path = cleanDisplayText(
    latestDiff?.path
    || latestTool?.file_path
    || latestTool?.params?.file_path
    || latestTool?.params?.path
    || latestTool?.params?.target_file
    || "",
    "",
  );
  if (!path) return "";

  const fileName = displayFileNameOnly(path);
  const added = Number(latestDiff?.added || 0) || 0;
  const removed = Number(latestDiff?.removed || 0) || 0;
  const animationOffset = "";
  const latestToolRunning = Boolean(latestTool && ["pending", "approved", "executing", "running"].includes(String(latestTool.status || "").toLowerCase()));

  return `
    <button
      class="codex-op-edit-artifact"
      type="button"
      data-open-workspace-file="${escapeHtml(path)}"
      data-open-workspace-line="1"
      data-open-workspace-column="1"
      title="${escapeHtml(path)}"
    >
      <span class="codex-op-edit-icon" aria-hidden="true">
        <svg viewBox="0 0 24 24" focusable="false" aria-hidden="true">
          <path d="M4 20l3.6-.7L18.3 8.6a1.8 1.8 0 0 0 0-2.6l-.3-.3a1.8 1.8 0 0 0-2.6 0L4.7 16.4 4 20z"></path>
          <path d="M13.8 7.3l2.9 2.9"></path>
        </svg>
      </span>
      <span class="codex-op-edit-prefix${latestToolRunning ? " is-streaming" : ""}" data-text="${escapeHtml(currentLanguage === "zh" ? "正在编辑" : "Editing")}"${latestToolRunning ? animationOffset : ""}>${escapeHtml(currentLanguage === "zh" ? "正在编辑" : "Editing")}</span>
      <span class="codex-op-edit-file">${escapeHtml(fileName)}</span>
      <span class="codex-op-edit-stats">
        <span class="is-added">+${escapeHtml(String(added))}</span>
        <span class="is-removed">-${escapeHtml(String(removed))}</span>
      </span>
    </button>
  `;
}

function operationArtifactMarkup(turn, label) {
  const text = cleanDisplayText(String(label || ""), "");
  if (!turn || !text) return "";
  if (!/(?:editing|created file|creating file|running|正在编辑|文件创建|创建文件|新建文件)/i.test(text)) {
    return operationEditArtifactMarkup(turn, label);
  }

  const diffs = Array.isArray(turn?.diffs) ? turn.diffs : [];
  const tools = Array.isArray(turn?.tools) ? turn.tools : [];
  const latestDiff = diffs.length ? diffs[diffs.length - 1] : null;
  const latestTool = [...tools].reverse().find((tool) => {
    const name = String(tool?.name || "").toLowerCase();
    return ["write_file", "apply_patch", "search_and_replace", "search_and_replace_multi", "rename_path", "mkdir"].includes(name);
  }) || null;
  const path = cleanDisplayText(
    latestDiff?.path
    || latestTool?.file_path
    || latestTool?.params?.file_path
    || latestTool?.params?.path
    || latestTool?.params?.target_file
    || "",
    "",
  );
  if (!path) return operationEditArtifactMarkup(turn, label);

  const fileName = displayFileNameOnly(path);
  const added = Number(latestDiff?.added || 0) || 0;
  const removed = Number(latestDiff?.removed || 0) || 0;
  const prefixText = /(?:created file|creating file|文件创建|创建文件|新建文件)/i.test(text)
    ? (currentLanguage === "zh" ? "文件创建" : "Created")
    : (currentLanguage === "zh" ? "正在编辑" : "Editing");
  const showStreaming = Boolean(
    latestTool
    && ["pending", "approved", "executing", "running"].includes(String(latestTool.status || "").toLowerCase()),
  );
  const animationOffset = "";

  return `
    <button
      class="codex-op-edit-artifact"
      type="button"
      data-open-workspace-file="${escapeHtml(path)}"
      data-open-workspace-line="1"
      data-open-workspace-column="1"
      title="${escapeHtml(path)}"
    >
      <span class="codex-op-edit-icon" aria-hidden="true">
        <svg viewBox="0 0 24 24" focusable="false" aria-hidden="true">
          <path d="M4 20l3.6-.7L18.3 8.6a1.8 1.8 0 0 0 0-2.6l-.3-.3a1.8 1.8 0 0 0-2.6 0L4.7 16.4 4 20z"></path>
          <path d="M13.8 7.3l2.9 2.9"></path>
        </svg>
      </span>
      <span class="codex-op-edit-prefix${showStreaming ? " is-streaming" : ""}" data-text="${escapeHtml(prefixText)}"${showStreaming ? animationOffset : ""}>${escapeHtml(prefixText)}</span>
      <span class="codex-op-edit-file">${escapeHtml(fileName)}</span>
      <span class="codex-op-edit-stats">
        <span class="is-added">+${escapeHtml(String(added))}</span>
        <span class="is-removed">-${escapeHtml(String(removed))}</span>
      </span>
    </button>
  `;
}

function buildOperationTimeline(turn, { isStreaming = false } = {}) {
  const items = [];
  const seenLabels = new Set();
  const pushItem = (label, timestamp = 0, extraClass = "") => {
    const text = cleanDisplayText(String(label || "").trim(), "");
    if (!text) return;
    if (/^(?:starting|execution|\u6b63\u5728\u6267\u884c|running|executing the current step)$/i.test(text) || /main agent is executing the current step/i.test(text)) return;
    const key = text.toLowerCase();
    if (seenLabels.has(key)) return;
    seenLabels.add(key);
    const attachment = operationArtifactMarkup(turn, text);
    items.push({
      timestamp: Number(timestamp || 0) || 0,
      body: `
        <div class="codex-op-row ${escapeHtml(extraClass)}">
          <span class="codex-op-dot" aria-hidden="true"></span>
          <div class="codex-op-main">
            <span class="codex-op-label">${escapeHtml(text)}</span>
            ${attachment}
          </div>
        </div>
      `,
    });
  };

  const progressNarration = summarizeOperationalText(String(turn?.progressNarration || "").trim(), "");
  if (progressNarration) {
    pushItem(progressNarration, 0, "is-primary");
  }

  const worklogEntries = Array.isArray(turn?.worklog) ? turn.worklog : [];
  worklogEntries.forEach((entry) => {
    const text = cleanDisplayText(entry?.text || "", "");
    if (!text) return;
    if (/\b(?:list_dir|find_files|read_file(?:_range)?|tree_dir|search_files|write_file|apply_patch|search_and_replace(?:_multi)?|rename_path|mkdir)\b/i.test(text)) return;
    if (progressNarration && (text === progressNarration || progressNarration.includes(text) || text.includes(progressNarration))) {
      return;
    }
    pushItem(text, Number(entry?.timestamp || 0) || 0);
  });

  const tools = Array.isArray(turn?.tools) ? turn.tools : [];
  const categoryMoments = new Map();
  tools.forEach((tool, index) => {
    const moment = describeToolMoment(tool);
    if (!moment) return;
    const existing = categoryMoments.get(moment.kind);
    const rank = moment.state === "fail" ? 3 : moment.state === "run" ? 2 : 1;
    const existingRank = existing?.state === "fail" ? 3 : existing?.state === "run" ? 2 : 1;
    if (!existing || rank >= existingRank) {
      categoryMoments.set(moment.kind, {
        ...moment,
        timestamp: Date.parse(String(tool?.updated_at || "")) || (Date.now() + index),
      });
    }
  });
  categoryMoments.forEach((moment) => {
    pushItem(moment.text, moment.timestamp, moment.state === "run" && isStreaming ? "is-active" : "");
  });

  const diffs = Array.isArray(turn?.diffs) ? turn.diffs : [];
  diffs.forEach((diff) => {
    if (categoryMoments.has("edit")) return;
    pushItem(
      summarizeRuntimeDiffNarration(diff),
      Date.parse(String(diff.updated_at || "")) || Number(diff.updated_at || 0) || Date.now(),
    );
  });

  const verifierReport = turn?.verifierReport || null;
  if (verifierReport) {
    pushItem(
      verifierReport.summary || (currentLanguage === "zh" ? "正在验证结果" : "Verifying results."),
      Date.now() + 1000,
    );
  }

  const subagents = dedupeSubagentEntries(turn?.subagents);
  subagents.forEach((subagent, index) => {
    pushItem(
      subagent.output || subagent.purpose || subagent.name || (currentLanguage === "zh" ? "子代理正在执行" : "Subagent running."),
      Date.parse(String(subagent.completed_at || subagent.started_at || "")) || (Date.now() + 2000 + index),
    );
  });

  return items
    .sort((a, b) => (Number(a.timestamp || 0) || 0) - (Number(b.timestamp || 0) || 0))
    .map((item) => item.body)
    .join("");
}

function buildOperationDetailPanels(turn, { isStreaming = false } = {}) {
  if (!turn) return "";
  const sections = [];

  const visibleProcess = Array.isArray(turn.process) ? turn.process.slice(-6) : [];
  if (visibleProcess.length) {
    sections.push(`
      <details class="codex-operation-detail-card">
        <summary>${escapeHtml(currentLanguage === "zh" ? "查看过程详情" : "Process details")}</summary>
        <div class="codex-process-list codex-steps-list">
          ${visibleProcess
            .map((event) => {
              const detailLine = event.detail
                ? `<div class="codex-process-detail">${escapeHtml(event.detail)}</div>`
                : "";
              const metaLine = event.meta
                ? `<div class="codex-process-meta">${escapeHtml(event.meta)}</div>`
                : "";
              return `
                <div class="codex-process-step codex-process-${escapeHtml(String(event.type || "activity"))}">
                  <div class="codex-step-rail" aria-hidden="true">
                    <span class="codex-step-dot"></span>
                  </div>
                  <div class="codex-process-main">
                    <div class="codex-process-label">${escapeHtml(event.label || "")}</div>
                    ${detailLine}
                    ${metaLine}
                  </div>
                </div>
              `;
            })
            .join("")}
        </div>
      </details>
    `);
  }

  const visibleTools = Array.isArray(turn.tools) ? turn.tools.slice(isStreaming ? -4 : -6) : [];
  if (visibleTools.length) {
    sections.push(`
      <details class="codex-operation-detail-card">
        <summary>${escapeHtml(currentLanguage === "zh" ? "查看工具详情" : "Tool details")}</summary>
        <div class="codex-tool-list codex-steps-list">
          ${visibleTools
            .map((tool, index) => {
              const argsPreview = compactToolArgs(tool.args);
              const verb = toolStatusVerb(tool.status);
              const pathLine = tool.file_path
                ? `<div class="codex-tool-path">${escapeHtml(tool.file_path)}</div>`
                : "";
              const argsLine = argsPreview
                ? `<div class="codex-tool-args">${escapeHtml(argsPreview)}</div>`
                : "";
              const resultLine =
                tool.status === "failed" && tool.result
                  ? `<div class="codex-tool-result is-error">${escapeHtml(String(tool.result).slice(0, 220))}</div>`
                  : "";
              const statusKey = String(tool.status || "pending");
              const active = isStreaming && index === visibleTools.length - 1 && ["pending", "approved", "executing"].includes(statusKey);
              return `
                <div class="codex-tool-card codex-tool-step codex-tool-${escapeHtml(statusKey)}${active ? " is-active" : ""}">
                  <div class="codex-step-rail" aria-hidden="true">
                    <span class="codex-step-dot"></span>
                  </div>
                  <div class="codex-tool-main">
                    <div class="codex-tool-title-row">
                      <span class="codex-tool-title">${escapeHtml(verb)}</span>
                      <span class="codex-tool-pill">${escapeHtml(toolStatusLabel(tool.status))}</span>
                    </div>
                    <div class="codex-tool-name">${escapeHtml(tool.name || "tool")}</div>
                    ${pathLine}
                    ${argsLine}
                    ${resultLine}
                  </div>
                </div>
              `;
            })
            .join("")}
        </div>
      </details>
    `);
  }

  return sections.join("");
}
function showToast(message) {
  if (!toast) return;
  toast.textContent = message;
  toast.classList.add("is-visible");
  window.clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => {
    toast.classList.remove("is-visible");
  }, 2200);
}


function classifyAppError(error, context = "generic") {
  const raw = String(error?.message || error || "").trim();
  const normalized = raw.toLowerCase();
  const zh = currentLanguage === "zh";

  const result = (kind, message, detail = raw) => ({
    kind,
    message,
    detail: detail || raw,
  });

  if (
    context === "send" && (
      !raw ||
      normalized.includes("current_session_id") ||
      normalized.includes("session not ready") ||
      normalized.includes("no active session") ||
      normalized.includes("session unavailable")
    )
  ) {
    return result(
      "session_not_ready",
      zh ? "会话还没准备好，请稍后再试。" : "Session is not ready yet. Please try again."
    );
  }

  if (
    normalized.includes("model streaming request failed") ||
    normalized.includes("api key") ||
    normalized.includes("api url") ||
    normalized.includes("provider supports streaming") ||
    normalized.includes("401") ||
    normalized.includes("403") ||
    normalized.includes("429") ||
    normalized.includes("400 bad request")
  ) {
    return result(
      "model_stream_failed",
      zh ? "模型流式连接失败，请检查 API URL、API Key、模型名和流式支持配置。" : "Model streaming connection failed. Check API URL, API key, model name, and streaming support."
    );
  }

  if (
    normalized.includes("sse error") ||
    normalized.includes("eventsource") ||
    normalized.includes("invalid status code") ||
    normalized.includes("stream failed")
  ) {
    return result(
      "sse_interrupted",
      zh ? "SSE 流被中断，请重试本轮或检查流式连接。" : "SSE stream was interrupted. Retry the turn or check the streaming connection."
    );
  }

  if (
    /tool\s+.+\s+failed/i.test(raw) ||
    normalized.includes("terminal_run failed") ||
    normalized.includes("run_command") ||
    normalized.includes("run_python") ||
    normalized.includes("tool call")
  ) {
    return result(
      "tool_execution_failed",
      zh ? "工具执行失败，请检查工具输出后重试。" : "Tool execution failed. Review the tool output and try again."
    );
  }

  if (
    normalized.includes("workspace file failed") ||
    normalized.includes("workspace file save failed") ||
    normalized.includes("undo failed") ||
    normalized.includes("reviewerror") ||
    normalized.includes("failed to load file diff") ||
    normalized.includes("open file")
  ) {
    return result(
      "workspace_operation_failed",
      zh ? "工作区操作失败，请检查文件状态后重试。" : "Workspace operation failed. Check the file state and try again."
    );
  }

  if (context === "send") {
    return result(
      "send_failed",
      zh ? "发送失败，请重试。" : "Send failed. Please try again."
    );
  }

  return result(
    "generic",
    zh ? "出现了一些问题，请重试。" : "Something went wrong. Please try again."
  );
}

function appErrorMessage(error, context = "generic", fallbackKey = "toastSendFailed") {
  const classified = classifyAppError(error, context);
  return classified?.message || t(fallbackKey);
}

function shouldShowSandboxNotice(sandbox) {
  if (!sandbox?.initialized || !sandbox?.first_run) return false;
  const root = String(sandbox.sandbox_root || "").trim();
  if (!root) return false;
  try {
    return localStorage.getItem(SANDBOX_NOTICE_KEY) !== root;
  } catch (_error) {
    return true;
  }
}

function markSandboxNoticeShown(sandbox) {
  const root = String(sandbox?.sandbox_root || "").trim();
  if (!root) return;
  try {
    localStorage.setItem(SANDBOX_NOTICE_KEY, root);
  } catch (_error) {
    // ignore storage failures
  }
}

function formatRunDependencyMessage(config) {
  const dependencies = Array.isArray(config?.missing_dependencies) ? config.missing_dependencies : [];
  const normalized = dependencies
    .map((item) => {
      const executable = String(item?.executable || "").trim();
      const configured = String(item?.configured || "").trim();
      if (!executable) return "";
      if (!configured || configured === executable) {
        return executable;
      }
      return `${executable} (${configured})`;
    })
    .filter(Boolean);
  const fallback = Array.isArray(config?.missing) ? config.missing.filter(Boolean) : [];
  const items = normalized.length ? normalized : fallback;
  if (!items.length) {
    return currentLanguage === "zh" ? "缺少运行时依赖。" : "Missing runtime dependency.";
  }
  const prefix = currentLanguage === "zh" ? "缺少可执行依赖:" : "Missing executable:";
  return `${prefix} ${items.join(", ")}`;
}

function parseAgentInputProtocol(rawContent) {
  const content = String(rawContent || "");
  const trimmed = content.trim();
  const inAgentMode = currentWorkspaceMode === "research";
  const forceResearch = /^\/spec(?:\s|$)/i.test(trimmed);
  if (forceResearch) {
    const stripped = trimmed.replace(/^\/spec(?:\s+)?/i, "").trim();
    return {
      outbound: stripped,
      display: stripped,
      mode: "research",
      forceResearch: true,
    };
  }
  if (!inAgentMode) {
    return {
      outbound: trimmed,
      display: trimmed,
      mode: "chat",
      forceResearch: false,
    };
  }

  return {
    outbound: trimmed,
    display: trimmed,
    mode: "agent",
    forceResearch: false,
  };
}

function formatSessionTime(value) {
  if (!value) return "";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "";
  const diffMs = Date.now() - date.getTime();
  const diffMinutes = Math.max(0, Math.floor(diffMs / 60000));
  if (diffMinutes < 30) {
    if (currentLanguage === "zh") {
      return diffMinutes <= 0 ? "刚刚" : `${diffMinutes} 分钟前`;
    }
    return diffMinutes <= 0 ? "just now" : `${diffMinutes} min ago`;
  }
  const formatter =
    currentLanguage === "zh"
      ? new Intl.DateTimeFormat("zh-CN", { month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit" })
      : new Intl.DateTimeFormat("en-US", { month: "short", day: "numeric", hour: "numeric", minute: "2-digit" });
  return formatter.format(date);
}

function formatActivityDuration() {
  if (!activityStartedAt) return "0s";
  const seconds = Math.max(0, Math.floor((Date.now() - activityStartedAt) / 1000));
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  const rest = seconds % 60;
  return `${minutes}m ${rest}s`;
}

function formatElapsedSince(timestamp) {
  const base = Number(timestamp || 0);
  if (!base) return "0s";
  const seconds = Math.max(0, Math.floor((Date.now() - base) / 1000));
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  const rest = seconds % 60;
  return `${minutes}m ${rest}s`;
}

function isNearMessageStreamBottom(threshold = 72) {
  if (!messageStream) return true;
  const distance = messageStream.scrollHeight - messageStream.scrollTop - messageStream.clientHeight;
  return distance <= threshold;
}

function scrollMessageStreamToBottom(force = false) {
  if (!messageStream) return;
  if (!force && !isNearMessageStreamBottom()) return;
  const target = Math.max(0, messageStream.scrollHeight - messageStream.clientHeight);
  const distance = target - messageStream.scrollTop;
  if (distance <= 1) {
    messageStream.scrollTop = target;
    return;
  }
  const prefersReducedMotion = typeof window.matchMedia === "function"
    && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (prefersReducedMotion || Math.abs(distance) > 720) {
    messageStream.scrollTop = target;
    return;
  }
  messageStreamFollowTarget = target;
  if (messageStreamFollowFrame != null) return;
  let lastTimestamp = 0;
  const tick = (timestamp) => {
    if (!messageStream) {
      messageStreamFollowFrame = null;
      return;
    }
    if (!lastTimestamp) lastTimestamp = timestamp;
    const delta = Math.min(32, timestamp - lastTimestamp || 16);
    lastTimestamp = timestamp;
    const currentTarget = Math.max(
      0,
      messageStream.scrollHeight - messageStream.clientHeight,
      messageStreamFollowTarget,
    );
    const remaining = currentTarget - messageStream.scrollTop;
    if (Math.abs(remaining) <= 1) {
      messageStream.scrollTop = currentTarget;
      messageStreamFollowFrame = null;
      return;
    }
    const follow = 1 - Math.pow(0.28, delta / 16);
    messageStream.scrollTop += remaining * follow;
    messageStreamFollowFrame = window.requestAnimationFrame(tick);
  };
  messageStreamFollowFrame = window.requestAnimationFrame(tick);
}

function waitForNextBrowserPaint() {
  return new Promise((resolve) => {
    window.requestAnimationFrame(() => {
      window.requestAnimationFrame(resolve);
    });
  });
}

function isVisibleSessionRunning() {
  const currentSessionId = String(bootstrapData?.current_session_id || "").trim();
  if (!currentSessionId) return false;
  return Boolean(getSessionRunState(currentSessionId)?.running);
}

function localStreamingSessionId(preferredSessionId = "") {
  const preferred = String(preferredSessionId || "").trim();
  if (preferred && getSessionRunState(preferred)?.running) {
    return preferred;
  }
  const streaming = String(currentStreamingSessionId || "").trim();
  if (streaming && getSessionRunState(streaming)?.running) {
    return streaming;
  }
  const visible = String(bootstrapData?.current_session_id || "").trim();
  if (visible && getSessionRunState(visible)?.running) {
    return visible;
  }
  return "";
}

function shouldPreserveStreamingConversationDom() {
  return Boolean(isVisibleSessionRunning() && pendingAssistantBubble && activeAssistantTurn);
}

function clearPendingAssistantFrames() {
  if (pendingAssistantTextFrame != null) {
    window.cancelAnimationFrame(pendingAssistantTextFrame);
    pendingAssistantTextFrame = null;
  }
  if (pendingAssistantStatusFrame != null) {
    window.cancelAnimationFrame(pendingAssistantStatusFrame);
    pendingAssistantStatusFrame = null;
  }
  if (pendingAssistantBubbleFrame != null) {
    window.cancelAnimationFrame(pendingAssistantBubbleFrame);
    pendingAssistantBubbleFrame = null;
  }
}

function combineAssistantSegments(existing, next) {
  const left = String(existing || "");
  const right = String(next || "");
  if (!left) return right;
  if (!right) return left;
  const leftTrimmed = sanitizeMessageContent(left).trim();
  const rightTrimmed = sanitizeMessageContent(right).trim();
  if (!leftTrimmed) return right;
  if (!rightTrimmed) return left;
  if (leftTrimmed === rightTrimmed) return left;
  if (leftTrimmed.includes(rightTrimmed)) return left;
  if (rightTrimmed.includes(leftTrimmed)) return right;
  const narrationMerged = mergeNarrationClauses(left, right);
  if (narrationMerged) {
    return narrationMerged;
  }
  const existingParagraphs = new Set(
    leftTrimmed
      .split(/\n{2,}/)
      .map((item) => sanitizeMessageContent(item).trim())
      .filter(Boolean),
  );
  const uniqueRight = right
    .split(/\n{2,}/)
    .map((item) => ({
      raw: item,
      normalized: sanitizeMessageContent(item).trim(),
    }))
    .filter((item) => item.normalized && !existingParagraphs.has(item.normalized))
    .map((item) => item.raw)
    .join("\n\n");
  if (!uniqueRight.trim()) {
    return left;
  }
  const markdownBlockBoundary =
    /(?:^|\n)\s*$/.test(left)
    || /^\s*(?:#{1,6}\s|[-*+]\s|\d+\.\s|```|>|---\s*$)/m.test(uniqueRight);
  const needsBreak =
    markdownBlockBoundary ||
    (!/[\n\s]$/.test(left) &&
    !/^[\n\s]/.test(uniqueRight));
  return needsBreak ? `${left}\n\n${uniqueRight}` : `${left}${uniqueRight}`;
}

function joinStreamingTextFragments(existing, next) {
  const left = String(existing || "");
  const right = String(next || "");
  if (!left) return right;
  if (!right) return left;
  if (/[\s\n]$/.test(left) || /^[\s\n]/.test(right)) {
    return `${left}${right}`;
  }
  if (/[([{"'`/_#-]$/.test(left) || /^[\])}"'`.,!?;:]/.test(right)) {
    return `${left}${right}`;
  }
  if (/[A-Za-z0-9]$/.test(left) && /^[A-Za-z0-9]/.test(right)) {
    return `${left} ${right}`;
  }
  return `${left}${right}`;
}

function mergeStreamingTextDelta(existing, incoming) {
  const left = String(existing || "");
  const right = String(incoming || "");
  if (!left) return right;
  if (!right) return left;
  if (right.startsWith(left)) return right;
  if (left.endsWith(right)) return left;

  const maxOverlap = Math.min(left.length, right.length);
  for (let overlap = maxOverlap; overlap > 0; overlap -= 1) {
    if (left.endsWith(right.slice(0, overlap))) {
      return `${left}${right.slice(overlap)}`;
    }
  }
  return `${left}${right}`;
}

function ensureActivityNodes() {
  if (!activityStrip) return null;
  if (!activityPillNode) {
    activityStrip.innerHTML = "";
    activityPillNode = document.createElement("div");
    activityPillNode.className = "activity-pill";
    const beat = document.createElement("span");
    beat.className = "activity-pill-beat";
    beat.setAttribute("aria-hidden", "true");
    activityPillLabelNode = document.createElement("span");
    activityPillLabelNode.className = "activity-pill-label";
    activityPillTimeNode = document.createElement("span");
    activityPillTimeNode.className = "activity-pill-time";
    activityPillNode.append(beat, activityPillLabelNode, activityPillTimeNode);
    activityStrip.appendChild(activityPillNode);
  }
  return activityPillNode;
}

function updateActivityPill() {
  const pill = ensureActivityNodes();
  if (!pill || !activityPillLabelNode || !activityPillTimeNode) return;
  activityStrip.hidden = false;
  activityStrip.setAttribute("data-active", "true");
  activityPillLabelNode.textContent = activityLabel;
  activityPillLabelNode.classList.add("is-live");
  activityPillTimeNode.textContent = activeAssistantTurn?.startedAt
    ? formatElapsedSince(activeAssistantTurn.startedAt)
    : formatActivityDuration();
}

function renderActivity() {
  if (!activityLabel) {
    stopActivity();
    return;
  }
  updateActivityPill();
}

function startActivity(label) {
  const nextLabel = String(label || "").trim();
  if (activityLabel !== nextLabel || !activityStartedAt) {
    activityStartedAt = Date.now();
  }
  activityLabel = nextLabel;
  window.clearInterval(activityTimer);
  activityTimer = window.setInterval(renderActivity, 240);
  renderActivity();
}

function setActivity(label) {
  activityLabel = label;
  renderActivity();
}

function stopActivity() {
  window.clearInterval(activityTimer);
  activityTimer = null;
  activityStartedAt = null;
  activityLabel = "";
  if (activityStrip) {
    activityStrip.hidden = true;
    activityStrip.innerHTML = "";
    activityStrip.removeAttribute("data-active");
  }
  activityPillNode = null;
  activityPillLabelNode = null;
  activityPillTimeNode = null;
  syncPendingAssistantStatus();
}

function currentEffortLabel() {
  return t(
    `effort${currentEffort.charAt(0).toUpperCase()}${currentEffort.slice(1).toLowerCase()}`,
  );
}

function updateEffortUI() {
  if (effortSlider && effortSlider.getAttribute("data-effort") !== currentEffort) {
    effortSlider.setAttribute("data-effort", currentEffort);
  }
  if (effortTriggerValue && effortTriggerValue.textContent !== currentEffortLabel()) {
    effortTriggerValue.textContent = currentEffortLabel();
  }
  if (effortPanelTitle) {
    const nextTitle = `${t("effortTitle")} ${currentEffortLabel()}`;
    if (effortPanelTitle.textContent !== nextTitle) {
      effortPanelTitle.textContent = nextTitle;
    }
  }
  if (effortPanelMeta) {
    const metaKey = `effortMeta${currentEffort.charAt(0).toUpperCase()}${currentEffort.slice(1).toLowerCase()}`;
    const nextMeta = t(metaKey);
    if (effortPanelMeta.textContent !== nextMeta) {
      effortPanelMeta.textContent = nextMeta;
    }
  }
  effortButtons.forEach((button) => {
    button.classList.toggle("is-active", (button.dataset.effort || "") === currentEffort);
  });
}

function closeEffortPanel() {
  if (effortDisclosure) {
    effortDisclosure.removeAttribute("open");
  }
}

function syncAutoApproveUI() {
  if (!riskBoundary) return;
  const enabled = Boolean(autoApproveTools?.checked);
  riskBoundary.classList.toggle("is-disabled", !enabled);
  riskBoundary
    .querySelectorAll(".segment")
    .forEach((button) => button.toggleAttribute("disabled", !enabled));
}

function setStopButtonVisible(visible) {
  if (!composerStop) return;
  composerStop.hidden = !visible;
}

function addProcessEvent(type, label, detail = "", meta = "", extra = {}) {
  const cleanLabel = String(label || "").trim();
  const cleanDetail = String(detail || "").trim();
  const cleanMeta = String(meta || "").trim();
  const cleanPhase = String(extra.phase || "").trim();
  const cleanStatus = String(extra.status || "").trim();
  const cleanAgent = String(extra.agent || "").trim();
  if (!cleanLabel && !cleanDetail) return;
  const placeholderActivity = `${cleanLabel} ${cleanDetail} ${cleanMeta}`.trim();
  if (
    /main agent is executing the current step/i.test(placeholderActivity)
    || (/^(?:starting|execution)$/i.test(cleanLabel) && !cleanMeta && (!cleanDetail || /executing the current step/i.test(cleanDetail)))
  ) {
    return;
  }

  const last = liveProcessEvents[liveProcessEvents.length - 1] || null;
  if (
    last &&
    last.type === type &&
    last.label === cleanLabel &&
    last.detail === cleanDetail &&
    last.meta === cleanMeta &&
    last.phase === cleanPhase &&
    last.status === cleanStatus &&
    last.agent === cleanAgent
  ) {
    last.timestamp = Date.now();
  } else {
    liveProcessEvents = [
      ...liveProcessEvents,
      {
        id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
        type,
        label: cleanLabel,
        detail: cleanDetail,
        meta: cleanMeta,
        phase: cleanPhase,
        status: cleanStatus,
        agent: cleanAgent,
        timestamp: Date.now(),
      },
    ].slice(-MAX_LIVE_PROCESS_EVENTS);
  }

  if (activeAssistantTurn) {
    activeAssistantTurn.process = liveProcessEvents.map((event) => ({
      id: event.id,
      type: event.type,
      label: event.label,
      detail: event.detail,
      meta: event.meta,
      phase: event.phase || "",
      status: event.status || "",
      agent: event.agent || "",
    }));
    const worklogEntry = describeActivityWorklog({
      label: cleanLabel,
      detail: cleanDetail,
      meta: cleanMeta,
      phase: cleanPhase,
      status: cleanStatus,
      agent: cleanAgent,
    });
    pushAssistantWorklog(worklogEntry);
    const processMoment = describeActivityMoment({
      type,
      label: cleanLabel,
      detail: cleanDetail,
      meta: cleanMeta,
      phase: cleanPhase,
      status: cleanStatus,
    });
    if (processMoment) {
      pushAssistantStreamMoment(processMoment);
    }
  }
  ensurePendingAssistantBubbleForRuntime();

  refreshPendingAssistantBubble();
  renderAgentProcessStrip();
}

function renderAgentProcessStrip() {
  if (!agentProcessStrip) return;
  if (pendingPermissionRequest) {
    agentProcessStrip.hidden = true;
    agentProcessStrip.innerHTML = "";
    syncPendingAssistantStatus();
    return;
  }
  const events = Array.isArray(liveProcessEvents) ? liveProcessEvents : [];
  if (!events.length) {
    agentProcessStrip.hidden = true;
    agentProcessStrip.innerHTML = "";
    syncPendingAssistantStatus();
    return;
  }
  const event = events[events.length - 1] || null;
  const firstPersonActivity = describeActivityWorklog({
    label: event?.label || "",
    detail: event?.detail || "",
    meta: event?.meta || "",
    phase: event?.phase || "",
    status: event?.status || "",
    agent: event?.agent || "",
  });
  const rawLabel = String(event?.label || event?.detail || "Running").trim();
  const labelMapZh = {
    starting: "准备中",
    planning: "规划中",
    execution: "执行中",
    delegation: "委派中",
    review: "审查中",
    verifier: "验证中",
    subagent: "子代理执行中",
    permission_required: "等待授权",
    editing: "编辑中",
    tool_complete: "工具已完成",
  };
  const labelMapEn = {
    starting: "Preparing",
    planning: "Planning",
    execution: "Executing",
    delegation: "Delegating",
    review: "Reviewing",
    verifier: "Verifying",
    subagent: "Subagent running",
    permission_required: "Permission required",
    editing: "Editing",
    tool_complete: "Tool complete",
  };
  const detailMapZh = {
    "Main agent is executing the current step": "主代理正在执行当前步骤",
    "Dispatching tool work": "正在分派工具任务",
    "Reviewer subagent is checking the turn": "审查子代理正在检查当前轮次",
  };
  const label = currentLanguage === "zh"
    ? (labelMapZh[rawLabel] || rawLabel)
    : (labelMapEn[rawLabel] || rawLabel);
  const detail = normalizeActivityDetail(firstPersonActivity?.text || "");
  const meta = [
    renderAgentName(event?.agent),
    renderActivityPhase(event?.phase),
    renderDelegateStatus(event?.status),
    cleanDisplayText(event?.meta || ""),
  ].filter(Boolean).join(" · ");
  const type = String(event?.type || "activity");
  agentProcessStrip.hidden = false;
  agentProcessStrip.innerHTML = `
    <div class="agent-process-inline">
      <div class="agent-process-inline-item agent-process-inline-${escapeHtml(type)}">
        <div class="agent-process-inline-label">${escapeHtml(label)}</div>
        ${detail ? `<div class="agent-process-inline-detail">${escapeHtml(detail)}</div>` : ""}
        ${meta ? `<div class="agent-process-inline-meta">${escapeHtml(meta)}</div>` : ""}
      </div>
    </div>
  `;
  syncPendingAssistantStatus();
}

function renderDelegateStatus(status) {
  const normalized = String(status || "").trim().toLowerCase();
  const labels = {
    planned: currentLanguage === "zh" ? "已规划" : "Planned",
    ready: currentLanguage === "zh" ? "就绪" : "Ready",
    running: currentLanguage === "zh" ? "执行中" : "Running",
    pass: currentLanguage === "zh" ? "通过" : "Pass",
    complete: currentLanguage === "zh" ? "完成" : "Complete",
    repair: currentLanguage === "zh" ? "修复中" : "Repair",
    failed: currentLanguage === "zh" ? "失败" : "Failed",
  };
  return labels[normalized] || status || (currentLanguage === "zh" ? "执行中" : "Running");
}

function renderAgentName(name) {
  const normalized = String(name || "").trim().toLowerCase();
  const labels = {
    main: currentLanguage === "zh" ? "主代理" : "Main",
    planner: currentLanguage === "zh" ? "规划器" : "Planner",
    reviewer: currentLanguage === "zh" ? "审查器" : "Reviewer",
    verifier: currentLanguage === "zh" ? "验证器" : "Verifier",
    repairer: currentLanguage === "zh" ? "修复器" : "Repairer",
    critic: currentLanguage === "zh" ? "评审器" : "Critic",
    researcher: currentLanguage === "zh" ? "研究器" : "Researcher",
    executor: currentLanguage === "zh" ? "执行器" : "Executor",
  };
  return labels[normalized] || cleanDisplayText(name, currentLanguage === "zh" ? "子代理" : "Subagent");
}

function renderActivityPhase(phase) {
  const normalized = String(phase || "").trim().toLowerCase();
  const labels = {
    initialize: currentLanguage === "zh" ? "初始化" : "Initialize",
    plan: currentLanguage === "zh" ? "规划" : "Plan",
    delegate: currentLanguage === "zh" ? "委派" : "Delegate",
    execute: currentLanguage === "zh" ? "执行" : "Execute",
    review: currentLanguage === "zh" ? "审查" : "Review",
    verify: currentLanguage === "zh" ? "验证" : "Verify",
    repair: currentLanguage === "zh" ? "修复" : "Repair",
    finalize: currentLanguage === "zh" ? "收尾" : "Finalize",
  };
  return labels[normalized] || phase || "";
}
function normalizeActivityDetail(detail) {
  const raw = cleanDisplayText(detail || "", "");
  if (!raw) return "";
  const normalized = raw.toLowerCase();
  if (/\b(?:list_dir|find_files|search_files|tree_dir|read_file|read_file_range)\b/.test(normalized)) {
    return zhLabel("正在查看工作区文件", "Viewing workspace files");
  }
  if (/\b(?:write_file|apply_patch|search_and_replace|search_and_replace_multi|rename_path|mkdir)\b/.test(normalized)) {
    return zhLabel("正在编辑文件", "Editing files");
  }
  if (/\bgit_[a-z0-9_]+\b/.test(normalized)) {
    return zhLabel("正在查看 Git 状态", "Checking Git status");
  }
  if (/main agent is executing the current step/i.test(raw)) {
    return zhLabel("正在执行当前步骤", "Executing the current step");
  }
  return raw;
}

function summarizeOperationalText(value, fallback = "") {
  const raw = cleanDisplayText(String(value || "").trim(), "");
  if (!raw) return cleanDisplayText(fallback, "");
  if (looksLikeDirectoryTreeDump(raw)) {
    return "Viewing workspace files";
  }
  if (looksLikeOperationalContentDump(raw)) {
    const normalized = raw.toLowerCase();
    if (/\b(?:list_dir|find_files|search_files|tree_dir|read_file|read_file_range)\b/.test(normalized)) {
      return "Viewing workspace files";
    }
    if (/\b(?:write_file|apply_patch|search_and_replace|search_and_replace_multi|rename_path|mkdir)\b/.test(normalized)) {
      return "Editing files";
    }
    if (/\bgit_[a-z0-9_]+\b/.test(normalized)) {
      return "Checking Git status";
    }
    return cleanDisplayText(fallback, "") || "Running tools";
  }
  return raw;
}

function displayFileNameOnly(filePath) {
  const raw = String(filePath || "").trim();
  if (!raw) return "";
  const normalized = raw.replace(/\\/g, "/");
  const parts = normalized.split("/").filter(Boolean);
  return parts[parts.length - 1] || raw;
}

function normalizeWorklogFingerprint(text) {
  return cleanDisplayText(text || "")
    .toLowerCase()
    .replace(/[`"'()[\]{}:;,._/\\|-]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

function isNearDuplicateWorklogEntry(candidateText, items, options = {}) {
  const fingerprint = normalizeWorklogFingerprint(candidateText);
  if (!fingerprint) return false;
  const recentItems = Array.isArray(items) ? items.slice(-(options.limit || 4)) : [];
  const now = Date.now();
  const maxAgeMs = Number.isFinite(options.maxAgeMs) ? options.maxAgeMs : 12000;
  const minLength = Number.isFinite(options.minLength) ? options.minLength : 10;
  return recentItems.some((entry) => {
    if (!entry) return false;
    if (Number.isFinite(entry.timestamp) && now - entry.timestamp > maxAgeMs) return false;
    const existingFingerprint = normalizeWorklogFingerprint(entry.text || "");
    if (!existingFingerprint) return false;
    if (existingFingerprint === fingerprint) return true;
    if (fingerprint.length < minLength || existingFingerprint.length < minLength) return false;
    return existingFingerprint.includes(fingerprint) || fingerprint.includes(existingFingerprint);
  });
}

function dedupeSubagentEntries(items) {
  const source = Array.isArray(items) ? items : [];
  if (!source.length) return [];
  const deduped = new Map();
  source.forEach((item, index) => {
    const key = String(item?.id || item?.name || "").trim() || `subagent-${index}`;
    deduped.set(key, { ...item, id: key });
  });
  return [...deduped.values()];
}

function pushTurnWorklogEntry(turn, entry) {
  if (!turn || !entry) return;
  const text = cleanDisplayText(entry.text || "");
  if (!text) return;
  const kind = String(entry.kind || "activity").trim() || "activity";
  const dedupeKey = String(entry.dedupeKey || `${kind}:${text}`).trim();
  const items = Array.isArray(turn.worklog) ? turn.worklog.slice() : [];
  const last = items[items.length - 1] || null;
  if (last && last.dedupeKey === dedupeKey) {
    last.timestamp = Date.now();
    turn.worklog = items.slice(-8);
    return;
  }
  items.push({
    id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    kind,
    text,
    dedupeKey,
    filePath: cleanDisplayText(entry.filePath || "", ""),
    added: Number(entry.added || 0) || 0,
    removed: Number(entry.removed || 0) || 0,
    timestamp: Date.now(),
  });
  turn.worklog = items.slice(-8);
}

function pushAssistantWorklog(entry) {
  if (!activeAssistantTurn || !entry) return;
  const text = cleanDisplayText(entry.text || "");
  if (!text) return;
  const kind = String(entry.kind || "activity").trim() || "activity";
  const dedupeKey = String(entry.dedupeKey || `${kind}:${text}`).trim();
  const items = Array.isArray(activeAssistantTurn.worklog) ? activeAssistantTurn.worklog.slice() : [];
  const last = items[items.length - 1] || null;
  if (last && last.dedupeKey === dedupeKey) {
    last.timestamp = Date.now();
    activeAssistantTurn.worklog = items.slice(-6);
    return;
  }
  if (isNearDuplicateWorklogEntry(text, items, { limit: 3, maxAgeMs: 10000, minLength: 12 })) {
    return;
  }
  items.push({
    id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    kind,
    text,
    dedupeKey,
    filePath: cleanDisplayText(entry.filePath || "", ""),
    added: Number(entry.added || 0) || 0,
    removed: Number(entry.removed || 0) || 0,
    timestamp: Date.now(),
  });
  activeAssistantTurn.worklog = items.slice(-6);
}

function pushTurnStreamMoment(turn, moment) {
  if (!turn || !moment) return;
  const text = cleanDisplayText(moment.text || "");
  if (!text) return;
  const kind = String(moment.kind || "note").trim() || "note";
  const dedupeKey = String(moment.dedupeKey || `${kind}:${text}`).trim();
  const operationKey = String(moment.operationKey || "").trim();
  const nextTimestamp = Number(moment.timestamp) || Date.now();
  const items = Array.isArray(turn.streamMoments) ? turn.streamMoments.slice() : [];
  const existing = operationKey
    ? [...items].reverse().find((item) => item.operationKey === operationKey)
    : items[items.length - 1]?.dedupeKey === dedupeKey
      ? items[items.length - 1]
      : null;
  if (existing) {
    existing.timestamp = nextTimestamp;
    existing.text = text;
    existing.kind = kind;
    existing.dedupeKey = dedupeKey;
    existing.state = String(moment.state || existing.state || "");
    existing.detail = cleanDisplayText(moment.detail || existing.detail || "", "");
    existing.filePath = cleanDisplayText(moment.filePath || existing.filePath || "", "");
    existing.added = Number(moment.added ?? existing.added ?? 0) || 0;
    existing.removed = Number(moment.removed ?? existing.removed ?? 0) || 0;
    turn.streamMoments = items.slice(-12);
    turn.lastStreamEventKind = kind;
    return;
  }
  items.push({
    id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    kind,
    text,
    detail: cleanDisplayText(moment.detail || "", ""),
    dedupeKey,
    operationKey,
    state: String(moment.state || "").trim(),
    filePath: cleanDisplayText(moment.filePath || "", ""),
    added: Number(moment.added || 0) || 0,
    removed: Number(moment.removed || 0) || 0,
    timestamp: nextTimestamp,
  });
  turn.streamMoments = items.slice(-12);
  turn.lastStreamEventKind = kind;
}

function pushAssistantStreamMoment(moment) {
  if (!activeAssistantTurn) return;
  pushTurnStreamMoment(activeAssistantTurn, moment);
  pendingAssistantOperationsDirty = true;
  ensurePendingAssistantBubbleForRuntime();
}

function completeContextCompactionMoment() {
  if (!activeAssistantTurn?.streamMoments?.length) return;
  const moment = [...activeAssistantTurn.streamMoments].reverse().find((item) => item?.kind === "compaction" && item?.state === "run");
  if (!moment) return;
  moment.state = "done";
  pendingAssistantStoryDirty = true;
}

function pushTurnTextSegment(turn, text, options = {}) {
  if (!turn) return;
  const cleanText = sanitizeMessageContent(String(text || ""));
  if (!cleanText.trim()) return false;
  const forceNew = Boolean(options.forceNew);
  const nextTimestamp = Number(options.timestamp) || Date.now();
  const items = Array.isArray(turn.textSegments) ? turn.textSegments.slice() : [];
  const last = items[items.length - 1] || null;
  if (!forceNew && last && turn.lastStreamEventKind === "text") {
    last.text = mergeStreamingTextDelta(String(last.text || ""), cleanText);
    last.timestamp = nextTimestamp;
    turn.textSegments = items.slice(-10);
    turn.textUpdatedAt = nextTimestamp;
    turn.lastStreamEventKind = "text";
    turn.text = turn.textSegments.map((item) => String(item?.text || "").trim()).filter(Boolean).join("\n\n");
    return false;
  }
  items.push({
    id: `${nextTimestamp}-${Math.random().toString(36).slice(2, 8)}`,
    text: cleanText,
    timestamp: nextTimestamp,
  });
  turn.textSegments = items.slice(-10);
  turn.textUpdatedAt = nextTimestamp;
  turn.lastStreamEventKind = "text";
  turn.text = turn.textSegments.map((item) => String(item?.text || "").trim()).filter(Boolean).join("\n\n");
  return true;
}

function replaceTurnTextSegments(turn, text, options = {}) {
  if (!turn) return;
  const cleanText = sanitizeMessageContent(String(text || ""));
  if (!cleanText.trim()) {
    turn.textSegments = [];
    turn.textUpdatedAt = Number(options.timestamp) || 0;
    turn.lastStreamEventKind = "";
    turn.text = "";
    return;
  }
  const nextTimestamp = Number(options.timestamp) || Date.now();
  turn.textSegments = [{
    id: `${nextTimestamp}-${Math.random().toString(36).slice(2, 8)}`,
    text: cleanText,
    timestamp: nextTimestamp,
  }];
  turn.textUpdatedAt = nextTimestamp;
  turn.lastStreamEventKind = "text";
  turn.text = cleanText.trim();
}

function pushAssistantProgressWorklogText(text) {
  const cleanText = normalizeAgentStageNarration(text);
  if (!cleanText) return;
  if (activeAssistantTurn) {
    const currentNarration = cleanDisplayText(String(activeAssistantTurn.progressNarration || "").trim(), "");
    activeAssistantTurn.progressNarration = currentNarration
      ? mergeAssistantText(currentNarration, cleanText)
      : cleanText;
  }
  const items = Array.isArray(activeAssistantTurn?.worklog) ? activeAssistantTurn.worklog : [];
  if (isNearDuplicateWorklogEntry(cleanText, items, { limit: 4, maxAgeMs: 14000, minLength: 8 })) {
    return;
  }
  const now = Date.now();
  const repeatedRecentProgress = items
    .slice(-3)
    .some((entry) => entry?.kind === "progress"
      && cleanDisplayText(entry.text || "") === cleanText
      && Number.isFinite(entry.timestamp)
      && now - entry.timestamp < 10000);
  if (repeatedRecentProgress) return;
  pushAssistantWorklog({
    kind: "progress",
    text: cleanText,
    dedupeKey: `progress:${cleanText}`,
  });
  const derivedMoment = extractAssistantOperationalMoment(cleanText) || {
    kind: "activity",
    text: cleanText,
    detail: "",
    state: "run",
    dedupeKey: `moment:progress:${normalizeText(cleanText)}`,
  };
  pushAssistantStreamMoment(derivedMoment);
}

function captureAssistantOperationNarration(text) {
  if (!activeAssistantTurn) return;
  const cleanText = cleanDisplayText(String(text || "").trim(), "");
  if (!cleanText) return;
  if (!assistantTextLooksLikeProcessNarration(cleanText)) return;
  const currentNarration = cleanDisplayText(String(activeAssistantTurn.progressNarration || "").trim(), "");
  if (!currentNarration || cleanText.includes(currentNarration) || cleanText.length > currentNarration.length) {
    activeAssistantTurn.progressNarration = cleanText;
  }
}

function ensurePendingAssistantBubbleForRuntime() {
  if (!activeAssistantTurn || pendingAssistantBubble) return;
  const currentSessionId = String(currentStreamingSessionId || bootstrapData?.current_session_id || "").trim();
  if (!getSessionRunState(currentSessionId)?.running) return;
  appendAssistantBubble(activeAssistantTurn.text || "");
}

function describeActivityWorklog(event) {
  const label = String(event?.label || "").trim();
  const detail = normalizeActivityDetail(event?.detail || "");
  const phase = String(event?.phase || "").trim().toLowerCase();
  const meta = cleanDisplayText(event?.meta || "");
  if (/main agent is executing the current step/i.test(`${label} ${detail} ${meta}`) || /^(?:starting|execution)$/i.test(label) && !detail && !meta) return null;
  if (!label && !detail && !meta) return null;
  if (!detail && !meta) return null;
  return {
    kind: phase || label || "activity",
    text: detail || meta,
    dedupeKey: `activity:${label}:${detail}:${meta}`,
  };
}

function describeActivityNarration(event) {
  return cleanDisplayText(describeActivityWorklog(event)?.text || "", "");
}

function describeActivityMoment(event) {
  const label = cleanDisplayText(event?.label || "", "");
  const detail = cleanDisplayText(event?.detail || event?.meta || "", "");
  const combined = `${label} ${detail}`.trim();
  if (!combined) return null;
  if (/main agent is executing the current step/i.test(combined) || /^(?:starting|execution)$/i.test(label) && !detail) return null;
  const status = String(event?.status || "").toLowerCase();
  const phase = String(event?.phase || "").toLowerCase();
  const failed = /fail|error|denied/.test(`${status} ${phase} ${combined}`);
  const done = /complete|completed|done|pass|success|succeeded|finished/.test(`${status} ${phase}`);
  const editing = /edit|write|patch|create|mkdir|rename|\u7f16\u8f91|\u5199\u5165|\u4fee\u6539|\u521b\u5efa|\u65b0\u5efa/.test(combined.toLowerCase());
  const inspection = /inspect|read|view|scan|search|check|review|\u67e5\u770b|\u8bfb\u53d6|\u626b\u63cf|\u641c\u7d22|\u68c0\u67e5|\u68c0\u89c6/.test(combined.toLowerCase());
  const checking = /verify|test|lint|build|cargo|node --check|\u9a8c\u8bc1|\u6d4b\u8bd5|\u6784\u5efa/.test(combined.toLowerCase());
  const state = failed ? "fail" : done ? "done" : "run";
  const text = checking
    ? (state === "run" ? zhLabel("\u6b63\u5728\u68c0\u67e5", "Checking") : state === "fail" ? zhLabel("\u68c0\u67e5\u5931\u8d25", "Check failed") : zhLabel("\u68c0\u67e5\u5b8c\u6210", "Check done"))
    : editing
      ? (state === "run" ? zhLabel("\u6b63\u5728\u7f16\u8f91", "Editing") : state === "fail" ? zhLabel("\u7f16\u8f91\u5931\u8d25", "Edit failed") : zhLabel("\u7f16\u8f91\u5b8c\u6210", "Edit done"))
      : inspection
        ? (state === "run" ? zhLabel("\u6b63\u5728\u67e5\u770b", "Inspecting") : state === "fail" ? zhLabel("\u67e5\u770b\u5931\u8d25", "Inspection failed") : zhLabel("\u67e5\u770b\u5b8c\u6210", "Inspection done"))
        : (state === "run" ? zhLabel("\u6b63\u5728\u6267\u884c", "Running") : state === "fail" ? zhLabel("\u5de5\u5177\u5931\u8d25", "Tool failed") : zhLabel("\u5de5\u5177\u5b8c\u6210", "Tool complete"));
  return {
    kind: checking ? "check" : editing ? "edit" : inspection ? "inspection" : "tool",
    text,
    detail: detail || label,
    state,
    dedupeKey: `moment:activity:${normalizeText(label || detail)}:${state}`,
  };
}

function normalizeAgentStageNarration(text) {
  return summarizeOperationalText(text, "Executing the current step");
}

function describeToolWorklog(tool) {
  if (!tool) return null;
  const name = cleanDisplayText(tool.name || "", currentLanguage === "zh" ? "工具" : "tool");
  const status = String(tool.status || "").trim().toLowerCase();
  const fileName = displayFileNameOnly(tool.file_path || "");
  const resultSummary = summarizeOperationalText(String(tool.result || "").trim(), "");
  if (status === "pending" || status === "running") {
    return {
      kind: "tool",
      text: fileName ? `${name} ${fileName}` : name,
      dedupeKey: `tool:${tool.call_id || name}:running:${fileName}`,
    };
  }
  if (status === "complete") {
    return {
      kind: "tool",
      text: resultSummary || summarizeRuntimeToolNarration(tool) || (fileName ? `${name} ${fileName}` : name),
      dedupeKey: `tool:${tool.call_id || name}:complete`,
    };
  }
  if (status === "failed") {
    return {
      kind: "tool",
      text: resultSummary || summarizeRuntimeToolNarration(tool) || (fileName ? `${name} ${fileName}` : name),
      dedupeKey: `tool:${tool.call_id || name}:failed`,
    };
  }
  return null;
}

function describeToolMoment(tool) {
  if (!tool) return null;
  const status = String(tool.status || "").trim().toLowerCase();
  const normalizedName = String(tool.name || "").trim().toLowerCase();
  const fileName = displayFileNameOnly(tool.file_path || tool.params?.file_path || tool.params?.path || tool.params?.target_file || "");
  const isCommand = isCommandLikeTool(normalizedName);
  const isInspection = ["list_dir", "find_files", "read_file", "read_file_range", "tree_dir", "search_files"].includes(normalizedName);
  const isEditing = ["write_file", "apply_patch", "search_and_replace", "search_and_replace_multi", "rename_path", "mkdir"].includes(normalizedName);
  const isChecking = normalizedName.startsWith("git_") || /(?:test|check|lint|build|verify)/.test(normalizedName);
  const kind = isCommand ? "command" : isEditing ? "edit" : isInspection ? "inspection" : isChecking ? "check" : "tool";
  const operationKey = `tool-category:${kind}`;
  const running = ["pending", "approved", "executing", "running"].includes(status);
  const failed = status === "failed" || status === "error" || status === "denied";
  const labels = {
    inspection: running ? zhLabel("\u6b63\u5728\u67e5\u770b", "Inspecting") : failed ? zhLabel("\u67e5\u770b\u5931\u8d25", "Inspection failed") : zhLabel("\u67e5\u770b\u5b8c\u6210", "Inspection done"),
    edit: running ? zhLabel("\u6b63\u5728\u7f16\u8f91", "Editing") : failed ? zhLabel("\u7f16\u8f91\u5931\u8d25", "Edit failed") : zhLabel("\u7f16\u8f91\u5b8c\u6210", "Edit done"),
    check: running ? zhLabel("\u6b63\u5728\u68c0\u67e5", "Checking") : failed ? zhLabel("\u68c0\u67e5\u5931\u8d25", "Check failed") : zhLabel("\u68c0\u67e5\u5b8c\u6210", "Check done"),
    command: running ? zhLabel("\u6267\u884c\u547d\u4ee4", "Running command") : failed ? zhLabel("\u547d\u4ee4\u6267\u884c\u5931\u8d25", "Command failed") : zhLabel("\u547d\u4ee4\u5b8c\u6210", "Command complete"),
    tool: running ? zhLabel("\u6b63\u5728\u6267\u884c", "Running tool") : failed ? zhLabel("\u5de5\u5177\u5931\u8d25", "Tool failed") : zhLabel("\u5de5\u5177\u5b8c\u6210", "Tool complete"),
  };
  return {
    kind,
    text: labels[kind],
    detail: fileName || "",
    state: failed ? "fail" : running ? "run" : "done",
    filePath: cleanDisplayText(tool.file_path || "", ""),
    operationKey,
    dedupeKey: `${operationKey}:${failed ? "fail" : running ? "run" : "done"}`,
  };
}

function describeEditedFileWorklog(file) {
  if (!file?.path) return null;
  return {
    kind: "edit",
    text: `${file.path} (+${Number(file.added || 0)} / -${Number(file.removed || 0)})`,
    dedupeKey: `edit:${file.path}:${Number(file.added || 0)}:${Number(file.removed || 0)}`,
    filePath: file.path,
    added: Number(file.added || 0) || 0,
    removed: Number(file.removed || 0) || 0,
  };
}

function describeEditedFileMoment(file) {
  if (!file?.path) return null;
  return {
    kind: "edit",
    text: zhLabel(`已编辑 ${displayFileNameOnly(file.path)}`, `Edited ${displayFileNameOnly(file.path)}`),
    detail: "",
    state: "done",
    filePath: file.path,
    added: Number(file.added || 0) || 0,
    removed: Number(file.removed || 0) || 0,
    operationKey: `edit:${file.path}`,
    dedupeKey: `moment:edit:${file.path}:${Number(file.added || 0)}:${Number(file.removed || 0)}`,
  };
}

function normalizedOperationMoment(moment, source = null) {
  if (!moment) return null;
  const kind = String(moment.kind || "tool").toLowerCase();
  const state = String(moment.state || "done").toLowerCase();
  const filePath = cleanDisplayText(moment.filePath || source?.path || source?.file_path || "", "");
  if (kind === "edit" && filePath) {
    const created = Number(moment.added || source?.added || 0) > 0
      && Number(moment.removed || source?.removed || 0) === 0
      && !String(source?.before_content || "").trim();
    return {
      ...moment,
      text: created
        ? zhLabel("\u6587\u4ef6\u521b\u5efa", "Created file")
        : state === "run"
          ? zhLabel("\u6b63\u5728\u7f16\u8f91", "Editing")
          : zhLabel("\u7f16\u8f91\u5b8c\u6210", "Edit done"),
      detail: "",
      filePath,
      dedupeKey: `${moment.dedupeKey || `moment:edit:${filePath}`}:${created ? "created" : state}`,
    };
  }
  if (kind === "command") {
    return {
      ...moment,
      text: state === "run"
        ? zhLabel("\u6267\u884c\u547d\u4ee4", "Running command")
        : state === "fail"
          ? zhLabel("\u547d\u4ee4\u6267\u884c\u5931\u8d25", "Command failed")
          : zhLabel("\u547d\u4ee4\u5b8c\u6210", "Command complete"),
    };
  }
  if (kind === "tool") {
    return {
      ...moment,
      text: state === "run"
        ? zhLabel("\u6b63\u5728\u6267\u884c", "Running tool")
        : state === "fail"
          ? zhLabel("\u5de5\u5177\u5931\u8d25", "Tool failed")
          : zhLabel("\u5de5\u5177\u5b8c\u6210", "Tool complete"),
    };
  }
  return moment;
}

function corroborateAssistantOperationalMoment(moment, turn) {
  if (!moment) return null;
  if (String(moment.kind || "").toLowerCase() !== "edit") return moment;
  const tools = Array.isArray(turn?.tools) ? turn.tools : [];
  const hasRealEditTool = tools.some((tool) => {
    const name = String(tool?.name || "").toLowerCase();
    return ["write_file", "apply_patch", "search_and_replace", "search_and_replace_multi", "rename_path", "mkdir"].includes(name);
  });
  if (hasRealEditTool || (Array.isArray(turn?.diffs) && turn.diffs.length > 0)) return moment;
  return {
    ...moment,
    kind: "activity",
    text: zhLabel("\u51c6\u5907\u7f16\u8f91", "Preparing to edit"),
    filePath: "",
    added: 0,
    removed: 0,
    dedupeKey: `moment:assistant-edit-intent:${normalizeText(moment.detail || moment.text || "")}`,
  };
}

function describeSubagentWorklog(subagent) {
  if (!subagent) return null;
  const name = cleanDisplayText(subagent.name || "", currentLanguage === "zh" ? "子代理" : "subagent");
  const status = String(subagent.status || "").trim().toLowerCase();
  const output = cleanDisplayText(String(subagent.output || "").slice(0, 120));
  if (status === "running") {
    return {
      kind: "subagent",
      text: name,
      dedupeKey: `subagent:${subagent.id || name}:running`,
    };
  }
  if (status === "complete" || status === "pass") {
    return {
      kind: "subagent",
      text: output || name,
      dedupeKey: `subagent:${subagent.id || name}:${status}:${output}`,
    };
  }
  return null;
}

function describeVerifierWorklog(report) {
  if (!report) return null;
  const status = String(report.status || "").trim().toLowerCase();
  const summary = cleanDisplayText(report.summary || "");
  if (status === "running") {
    return {
      kind: "verifier",
      text: summary || status,
      dedupeKey: "verifier:running",
    };
  }
  if (status === "pass" || status === "complete") {
    return {
      kind: "verifier",
      text: summary || status,
      dedupeKey: `verifier:${status}:${summary}`,
    };
  }
  if (status === "repair" || status === "failed") {
    return {
      kind: "verifier",
      text: summary || status,
      dedupeKey: `verifier:${status}:${summary}`,
    };
  }
  return null;
}

function describeCompletionWorklog(event) {
  const detail = cleanDisplayText(event?.activity?.detail || "");
  if (!detail) return null;
  return {
    kind: "complete",
    text: detail,
    dedupeKey: `complete:${detail}`,
  };
}

function renderAgentRuntimeStrip() {
  if (!agentRuntimeStrip) return;
  const currentSessionId = String(currentStreamingSessionId || bootstrapData?.current_session_id || "").trim();
  const isRunning = Boolean(getSessionRunState(currentSessionId)?.running);
  const recentFiles = Array.isArray(liveEditedFiles) ? liveEditedFiles.filter((item) => item?.path) : [];
  const fallbackFiles = Array.isArray(activeAssistantTurn?.diffs)
    ? activeAssistantTurn.diffs
        .filter((item) => item?.path)
        .map((item) => ({
          path: item.path,
          added: Number(item.added || 0),
          removed: Number(item.removed || 0),
        }))
    : [];
  const sourceFiles = recentFiles.length ? recentFiles : fallbackFiles;
  const file = sourceFiles.length
    ? sourceFiles[sourceFiles.length - 1]
    : null;

  if (!isRunning || !file) {
    agentRuntimeStrip.hidden = true;
    agentRuntimeStrip.innerHTML = "";
    return;
  }

  if (activeAssistantTurn && file) {
    upsertDiffEntry(file);
  }

  const orderedFiles = sourceFiles.slice(-3).reverse();
  agentRuntimeStrip.hidden = false;
  agentRuntimeStrip.innerHTML = `
    <div class="agent-runtime-chip-wrap">
      ${orderedFiles.map((item, index) => `
        <button
          class="agent-runtime-chip agent-runtime-chip-action${index === 0 ? " is-active" : ""}"
          type="button"
          data-open-workspace-file="${escapeHtml(item.path || "")}"
          data-open-workspace-line="1"
          data-open-workspace-column="1"
        >
          <span class="agent-runtime-label">${escapeHtml(currentLanguage === "zh" ? "正在编辑" : "Editing")}</span>
          <div class="agent-runtime-value">
            <span class="agent-runtime-path">${escapeHtml(displayFileNameOnly(item.path || ""))}</span>
            <span class="agent-runtime-stats">+${escapeHtml(String(item.added || 0))} / -${escapeHtml(String(item.removed || 0))}</span>
          </div>
        </button>
      `).join("")}
    </div>
  `;
  bindTurnInteractionHandlers(agentRuntimeStrip);
}

function renderPermissionStrip() {
  if (!permissionStrip) return;
  if (!pendingPermissionRequest) {
    permissionStrip.hidden = true;
    permissionStrip.innerHTML = "";
    if (activeAssistantTurn) {
      activeAssistantTurn.permission = null;
    }
    schedulePendingAssistantTextSync();
    return;
  }

  if (activeAssistantTurn) {
    activeAssistantTurn.permission = pendingPermissionRequest;
  }
  permissionStrip.hidden = false;
  permissionStrip.innerHTML = `
    <div class="permission-card">
      <div>
        <div class="permission-title">${escapeHtml(currentLanguage === "zh" ? "等待工具授权" : "Awaiting tool approval")}</div>
      </div>
      <pre class="permission-args">${escapeHtml(JSON.stringify(pendingPermissionRequest.args || {}, null, 2))}</pre>
      <div class="permission-actions">
        <button class="permission-button" type="button" data-permission-action="deny">${escapeHtml(currentLanguage === "zh" ? "拒绝" : "Deny")}</button>
      </div>
    </div>
  `;
  bindTurnInteractionHandlers(permissionStrip);
  schedulePendingAssistantTextSync();
}

function setTerminalDrawerVisible(visible) {
  if (!terminalDrawer) return;
  terminalDrawer.hidden = !visible;
  terminalDrawer.classList.toggle("is-open", Boolean(visible));
  scheduleTerminalPoll(visible ? 1500 : 3000);
}

function getActiveTerminal() {
  const sessions = Array.isArray(terminalState?.sessions) ? terminalState.sessions : [];
  if (!sessions.length) return null;
  return sessions.find((session) => session.id === terminalState.active_id) || sessions[0] || null;
}

function renderTerminalDrawer() {
  if (!terminalDrawer || !terminalTabList || !terminalOutput || !terminalInput) return;
  const sessions = Array.isArray(terminalState?.sessions) ? terminalState.sessions : [];
  const active = getActiveTerminal();

  if (!sessions.length) {
    terminalDrawerDismissed = false;
  }
  setTerminalDrawerVisible(sessions.length > 0 && !terminalDrawerDismissed);

  terminalTabList.innerHTML = sessions
    .map((session) => `
      <button class="terminal-tab${session.id === active?.id ? " is-active" : ""}" type="button" data-terminal-tab="${escapeHtml(session.id)}">
        <span class="terminal-tab-title">${escapeHtml(session.title || session.id)}</span>
        <span class="terminal-tab-time">${escapeHtml(session.created_at || "")}</span>
        <span class="terminal-tab-close" data-terminal-close="${escapeHtml(session.id)}">x</span>
      </button>
    `)
    .join("");

  terminalOutput.textContent = active?.buffer || "";
  terminalInput.disabled = !active;
  terminalInput.placeholder = active ? active.cwd || "" : "";
  if (terminalScreen) {
    terminalScreen.scrollTop = terminalScreen.scrollHeight;
  }

  terminalTabList.querySelectorAll("[data-terminal-tab]").forEach((button) => {
    button.addEventListener("click", (event) => {
      if (event.target instanceof HTMLElement && event.target.hasAttribute("data-terminal-close")) {
        return;
      }
      terminalState.active_id = button.getAttribute("data-terminal-tab") || terminalState.active_id;
      renderTerminalDrawer();
      terminalInput.focus();
    });
  });

  terminalTabList.querySelectorAll("[data-terminal-close]").forEach((button) => {
    button.addEventListener("click", async (event) => {
      event.stopPropagation();
      const terminalId = button.getAttribute("data-terminal-close") || "";
      if (!terminalId) return;
      try {
        await closeTerminal(terminalId);
      } catch (error) {
        console.error(error);
        showToast(appErrorMessage(error, "workspace", "toastSendFailed"));
      }
    });
  });
}

function openSettingsPanel(panelId) {
  activeSettingsPanel = panelId;
  settingsPanels.forEach((panel) => {
    panel.hidden = panel.id !== panelId;
  });
}

function closeSettingsPanels() {
  activeSettingsPanel = null;
  settingsPanels.forEach((panel) => {
    panel.hidden = true;
  });
}

function setActivityPanel(panelId, { preserveMainView = false } = {}) {
  const nextPanel = panelId || null;
  activeActivityPanel = nextPanel;
  if (nextPanel) {
    preferredLeftActivityPanel = nextPanel;
  }
  activityRailButtons.forEach((button) => {
    button.classList.toggle("is-active", button.dataset.activityPanel === nextPanel);
  });
  activityPanels.forEach((panel) => {
    panel.classList.toggle("is-active", panel.dataset.activityPanelId === nextPanel);
  });
  if (activityFlyout) {
    activityFlyout.classList.toggle("is-open", Boolean(nextPanel));
  }
  if (appShell) {
    appShell.classList.toggle("has-activity-flyout", Boolean(nextPanel));
  }
  activityFlyout?.classList.toggle("is-search-github-preview", isSearchGitHubPreviewActive());
  appShell?.classList.toggle("has-search-github-preview", isSearchGitHubPreviewActive());

  if (!preserveMainView) {
    if (nextPanel === "git") {
      setMainView("git");
    } else if (currentMainView === "git" && nextPanel !== "git") {
      setMainView("chat");
    }
  }
  syncShellLayoutVars();
  syncLayoutCornerControls();
}

function renderExtensionList(query = "") {
  if (!extensionList) return;
  const keyword = String(query || "").trim().toLowerCase();
  const items = extensionCatalog.filter((item) => {
    if (!keyword) return true;
    return [item.title, item.meta, item.description].some((value) =>
      String(value || "").toLowerCase().includes(keyword),
    );
  });

  extensionList.innerHTML = items.length
    ? items
        .map(
          (item) => `
            <button class="extension-card" type="button" data-extension-id="${escapeHtml(item.id)}">
              <span class="extension-card-title">${escapeHtml(item.title)}</span>
              <span class="extension-card-meta">${escapeHtml(item.meta)}</span>
              <span class="extension-card-desc">${escapeHtml(item.description)}</span>
            </button>
          `,
        )
        .join("")
    : `<div class="git-empty">${escapeHtml(currentLanguage === "zh" ? "\u6ca1\u6709\u5339\u914d\u7684\u6269\u5c55\u3002" : "No matching extensions.")}</div>`;
}

function searchPlaceholderForMode(mode = searchMode) {
  if (mode === "papers") return t("searchPlaceholderPapers");
  if (mode === "tracking") return t("searchPlaceholderTracking");
  if (mode === "benchmarks") return t("searchPlaceholderBenchmarks");
  if (mode === "models") return t("searchPlaceholderModels");
  if (mode === "datasets") return t("searchPlaceholderDatasets");
  if (mode === "github") return t("searchPlaceholderGitHub");
  return t("searchPlaceholderWeb");
}

function searchStatusLabel(status) {
  if (status === "ready") return t("searchHealthReady");
  if (status === "degraded") return t("searchHealthDegraded");
  if (status === "down") return t("searchHealthDown");
  return t("searchHealthUnknown");
}

function searchStatusClass(status) {
  if (status === "ready") return "is-ready";
  if (status === "degraded") return "is-degraded";
  if (status === "down") return "is-down";
  return "is-unknown";
}

function normalizeSearchResults(mode, payload) {
  if (!payload || typeof payload !== "object") return [];
  if (mode === "papers") return Array.isArray(payload.results) ? payload.results : [];
  if (mode === "tracking") return Array.isArray(payload.results) ? payload.results : [];
  if (mode === "benchmarks") return Array.isArray(payload.results) ? payload.results : [];
  if (mode === "models") return Array.isArray(payload.results) ? payload.results : [];
  if (mode === "datasets") return Array.isArray(payload.datasets) ? payload.datasets : [];
  if (mode === "github") {
    if (Array.isArray(payload.results)) return payload.results;
    if (Array.isArray(payload?.fallback?.results)) return payload.fallback.results;
    return [];
  }
  return Array.isArray(payload.results) ? payload.results : [];
}

function normalizeSearchHints(payload) {
  if (!payload || typeof payload !== "object") return [];
  return Array.isArray(payload.hints) ? payload.hints.filter(Boolean).map((item) => cleanDisplayText(item)) : [];
}

function githubPreviewSelectionKey(item) {
  const repo = cleanDisplayText(
    item?.repository_full_name || item?.full_name || item?.html_url || item?.repository_url || "",
    "",
  );
  const path = cleanDisplayText(item?.path || "", "");
  return path ? `${repo}::${path}` : repo;
}

function githubPreviewParentPath(path) {
  const normalized = cleanDisplayText(path || "", "").replace(/^\/+|\/+$/g, "");
  if (!normalized) return "";
  const parts = normalized.split("/").filter(Boolean);
  parts.pop();
  return parts.join("/");
}

function currentGitHubPreviewSourceItem() {
  const items = normalizeSearchResults("github", searchState.results);
  const index = Number(searchState.githubPreviewSourceIndex ?? -1);
  if (!Number.isFinite(index) || index < 0 || index >= items.length) return null;
  return items[index] || null;
}

function normalizeGitHubPreviewCommit(commitSha) {
  return cleanDisplayText(commitSha || "", "") || null;
}

function normalizeGitHubCompareSelection(compareBaseSha, compareHeadSha, fallbackHeadSha = null) {
  const baseSha = normalizeGitHubPreviewCommit(compareBaseSha);
  const headSha = normalizeGitHubPreviewCommit(compareHeadSha) || normalizeGitHubPreviewCommit(fallbackHeadSha);
  return { baseSha, headSha };
}

function buildGitHubSideBySideRows(lines) {
  const source = Array.isArray(lines) ? lines : [];
  const rows = [];
  let index = 0;
  while (index < source.length) {
    const line = source[index];
    const kind = cleanDisplayText(line?.kind || "context", "context");
    if (kind === "removed") {
      const removed = [];
      while (index < source.length && cleanDisplayText(source[index]?.kind || "", "") === "removed") {
        removed.push(source[index]);
        index += 1;
      }
      const added = [];
      while (index < source.length && cleanDisplayText(source[index]?.kind || "", "") === "added") {
        added.push(source[index]);
        index += 1;
      }
      const width = Math.max(removed.length, added.length);
      for (let cursor = 0; cursor < width; cursor += 1) {
        rows.push({
          kind: removed[cursor] && added[cursor]
            ? "changed"
            : removed[cursor]
              ? "removed"
              : "added",
          left: removed[cursor] || null,
          right: added[cursor] || null,
        });
      }
      continue;
    }
    if (kind === "added") {
      rows.push({
        kind: "added",
        left: null,
        right: line,
      });
      index += 1;
      continue;
    }
    rows.push({
      kind: "context",
      left: line,
      right: line,
    });
    index += 1;
  }
  return rows;
}

function renderGitHubPreviewDiff(diff, options = {}) {
  if (!diff || typeof diff !== "object" || diff.available !== true) {
    const detail = cleanDisplayText(diff?.detail || "", "");
    return `<div class="search-preview-empty">${escapeHtml(detail || zhLabel("当前选择没有可用的 commit diff。", "No commit diff is available for the current selection."))}</div>`;
  }
  const hunks = Array.isArray(diff.hunks) ? diff.hunks : [];
  if (!hunks.length) {
    const detail = cleanDisplayText(diff?.detail || "", "");
    return `<div class="search-preview-empty">${escapeHtml(detail || zhLabel("当前选择没有可用的 diff hunk。", "No diff hunks are available for the current selection."))}</div>`;
  }
  const sideBySide = options.sideBySide === true;
  return hunks.map((hunk) => {
    const rows = sideBySide
      ? buildGitHubSideBySideRows(hunk.lines).map((row) => `
        <div class="search-preview-diff-side-row is-${escapeHtml(cleanDisplayText(row.kind || "context", "context"))}">
          <span class="review-code-gutter">${row.left?.old_number ?? ""}</span>
          <span class="search-preview-diff-side-content is-left">${escapeHtml(displayPlainText(row.left?.content || "", row.left?.content || ""))}</span>
          <span class="review-code-gutter">${row.right?.new_number ?? ""}</span>
          <span class="search-preview-diff-side-content is-right">${escapeHtml(displayPlainText(row.right?.content || "", row.right?.content || ""))}</span>
        </div>
      `).join("")
      : (Array.isArray(hunk.lines) ? hunk.lines : []).map((line) => {
          const kind = cleanDisplayText(line?.kind || "context", "context");
          return `
            <div class="review-code-row is-${escapeHtml(kind)}">
              <span class="review-code-gutter">${line?.old_number ?? ""}</span>
              <span class="review-code-gutter">${line?.new_number ?? ""}</span>
              <span class="review-code-content">${escapeHtml(displayPlainText(line?.content || "", line?.content || ""))}</span>
            </div>
          `;
        }).join("");
    return `
      <section class="search-preview-diff-hunk">
        <div class="review-hunk-header">${escapeHtml(cleanDisplayText(hunk.header || "", ""))}</div>
        <div class="${sideBySide ? "search-preview-diff-side-grid" : "review-code"}">${rows}</div>
      </section>
    `;
  }).join("");
}

async function loadGitHubPreview(repoFullName, branch = null, path = null, options = {}) {
  const normalizedRepo = cleanDisplayText(repoFullName || "", "");
  if (!normalizedRepo) return;
  const normalizedBranch = cleanDisplayText(branch || "", "") || null;
  const normalizedPath = cleanDisplayText(path || "", "") || null;
  const normalizedCommitSha = normalizeGitHubPreviewCommit(options.commitSha);
  const normalizedCompareBaseSha = normalizeGitHubPreviewCommit(options.compareBaseSha);
  const normalizedCompareHeadSha = normalizeGitHubPreviewCommit(options.compareHeadSha);
  const normalizedHistoryScopeMode = cleanDisplayText(options.historyScopeMode || "", "") || null;
  searchState.githubPreviewLoading = true;
  searchState.githubPreviewError = "";
  renderSearchPanel();
  try {
    const response = await hostClient.search.githubPreview(
      normalizedRepo,
      normalizedBranch,
      normalizedPath,
      normalizedCommitSha,
      normalizedCompareBaseSha,
      normalizedCompareHeadSha,
      normalizedHistoryScopeMode,
    );
    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(errorText || `github preview failed: ${response.status}`);
    }
    const payload = await response.json();
    searchState.githubPreview = payload?.data || payload || null;
    if (Number.isFinite(options.sourceIndex)) {
      searchState.githubPreviewSourceIndex = Number(options.sourceIndex);
    }
    const historyEntry = {
      repoFullName: normalizedRepo,
      branch: normalizedBranch,
      path: normalizedPath,
      commitSha: normalizedCommitSha,
      compareBaseSha: normalizedCompareBaseSha,
      compareHeadSha: normalizedCompareHeadSha,
      historyScopeMode: normalizedHistoryScopeMode,
      selectionKey: cleanDisplayText(searchState.githubPreview?.selection_key || "", ""),
    };
    const shouldPushHistory = options.pushHistory !== false;
    const currentHistory = searchState.githubPreviewHistory[searchState.githubPreviewHistoryIndex] || null;
    if (shouldPushHistory) {
      const sameAsCurrent = currentHistory
        && currentHistory.repoFullName === historyEntry.repoFullName
        && currentHistory.branch === historyEntry.branch
        && currentHistory.path === historyEntry.path
        && currentHistory.commitSha === historyEntry.commitSha
        && currentHistory.compareBaseSha === historyEntry.compareBaseSha
        && currentHistory.compareHeadSha === historyEntry.compareHeadSha;
      if (!sameAsCurrent) {
        searchState.githubPreviewHistory = searchState.githubPreviewHistory.slice(0, searchState.githubPreviewHistoryIndex + 1);
        searchState.githubPreviewHistory.push(historyEntry);
        searchState.githubPreviewHistoryIndex = searchState.githubPreviewHistory.length - 1;
      }
    } else if (Number.isFinite(options.historyIndex)) {
      searchState.githubPreviewHistoryIndex = Number(options.historyIndex);
    }
  } catch (error) {
    searchState.githubPreview = null;
    searchState.githubPreviewError = cleanDisplayText(error?.message || "", "") || t("searchError");
    if (options.resetSourceIndex !== false) {
      searchState.githubPreviewSourceIndex = Number.isFinite(options.sourceIndex) ? Number(options.sourceIndex) : -1;
    }
  } finally {
    searchState.githubPreviewLoading = false;
    renderSearchPanel();
  }
}

function githubPreviewCanGoBack() {
  return Number(searchState.githubPreviewHistoryIndex) > 0;
}

function githubPreviewCanGoForward() {
  return Number(searchState.githubPreviewHistoryIndex) >= 0
    && Number(searchState.githubPreviewHistoryIndex) < searchState.githubPreviewHistory.length - 1;
}

function githubPreviewHistoryLabel(entry) {
  if (!entry || typeof entry !== "object") return "";
  const repo = cleanDisplayText(entry.repoFullName || "", "");
  const path = cleanDisplayText(entry.path || "", "");
  const commitSha = cleanDisplayText(entry.commitSha || "", "");
  const compareBaseSha = cleanDisplayText(entry.compareBaseSha || "", "");
  const compareHeadSha = cleanDisplayText(entry.compareHeadSha || "", "");
  const base = path ? `${repo}:${path}` : repo;
  if (compareBaseSha && compareHeadSha) {
    return `${base}@${compareBaseSha.slice(0, 7)}..${compareHeadSha.slice(0, 7)}`;
  }
  return commitSha ? `${base}@${commitSha.slice(0, 7)}` : base;
}

function renderSearchPreviewPanel() {
  if (!searchPreviewPanel) return;
  if (searchMode !== "github") {
    searchPreviewPanel.innerHTML = "";
    return;
  }
  if (searchState.githubPreviewLoading) {
    searchPreviewPanel.innerHTML = `<div class="search-preview-empty">${escapeHtml(t("searchLoading"))}</div>`;
    return;
  }
  if (searchState.githubPreviewError) {
    searchPreviewPanel.innerHTML = `<div class="search-preview-empty">${escapeHtml(searchState.githubPreviewError)}</div>`;
    return;
  }
  const preview = searchState.githubPreview;
  if (!preview || typeof preview !== "object") {
    searchPreviewPanel.innerHTML = `<div class="search-preview-empty">${escapeHtml(zhLabel("选择一个 GitHub 结果以查看 README、目录树和文件命中。", "Select a GitHub result to inspect README, tree, and file hits."))}</div>`;
    return;
  }
  const repository = preview.repository || {};
  const entries = Array.isArray(preview.entries) ? preview.entries.slice(0, 16) : [];
  const readme = preview.readme && typeof preview.readme === "object" ? preview.readme : null;
  const selectedFile = preview.selected_file && typeof preview.selected_file === "object" ? preview.selected_file : null;
  const history = preview.history && typeof preview.history === "object" ? preview.history : {};
  const commitList = Array.isArray(history.commits) ? history.commits : [];
  const selectedCommit = history.selected_commit && typeof history.selected_commit === "object" ? history.selected_commit : null;
  const selectedCommitSha = cleanDisplayText(history.selected_commit_sha || selectedCommit?.sha || "", "");
  const historyScopeMode = cleanDisplayText(history.scope_mode || "", "") || "selection";
  const selectedDiff = history.diff && typeof history.diff === "object" ? history.diff : null;
  const compareSelection = normalizeGitHubCompareSelection(
    cleanDisplayText(history.compare_base_sha || "", ""),
    cleanDisplayText(history.compare_head_sha || "", ""),
    selectedCommitSha,
  );
  const compareBaseSha = compareSelection.baseSha;
  const compareHeadSha = compareSelection.headSha;
  const compare = history.compare && typeof history.compare === "object" ? history.compare : null;
  const compareFiles = Array.isArray(compare?.files) ? compare.files : [];
  const sourceItem = currentGitHubPreviewSourceItem();
  const textMatches = Array.isArray(sourceItem?.text_matches)
    ? sourceItem.text_matches.map((item) => cleanDisplayText(item || "", "")).filter(Boolean).slice(0, 3)
    : [];
  const branch = cleanDisplayText(repository.default_branch || "", "");
  const activeRef = cleanDisplayText(preview.active_ref || branch, "") || branch;
  const activeRefKind = cleanDisplayText(preview.active_ref_kind || "branch", "branch");
  const previewPath = cleanDisplayText(preview.path || "", "");
  const parentPath = githubPreviewParentPath(
    selectedFile && cleanDisplayText(selectedFile.path || "", "")
      ? cleanDisplayText(selectedFile.path || "", "")
      : previewPath,
  );
  const repoUrl = cleanDisplayText(repository.html_url || "", "");
  const selectedFileHtml = cleanDisplayText(selectedFile?.html_url || "", "");
  const selectedFileRaw = cleanDisplayText(selectedFile?.download_url || "", "");
  const selectedFileContent = String(selectedFile?.content || "");
  const selectedFileLanguage = cleanDisplayText(selectedFile?.language || selectedFile?.path || "text", "text");
  const readmeHtml = cleanDisplayText(readme?.html_url || "", "");
  const readmeRaw = cleanDisplayText(readme?.download_url || "", "");
  const readmeContent = String(readme?.content || "");
  const readmeLanguage = cleanDisplayText(readme?.language || readme?.path || "markdown", "markdown");
  const pathLabel = previewPath || cleanDisplayText(selectedFile?.path || "", "") || "/";
  const historyEntry = searchState.githubPreviewHistory[searchState.githubPreviewHistoryIndex] || null;
  const activeCommitContextSha = selectedCommitSha || compareHeadSha || null;
  const historyPath = cleanDisplayText(history.scope_path || selectedFile?.path || preview.path || "", "");
  const historyTitle = selectedFile && cleanDisplayText(selectedFile.path || "", "")
    ? historyScopeMode === "repository"
      ? zhLabel("仓库提交历史", "Repository commit history")
      : zhLabel("文件提交历史", "File commit history")
    : historyScopeMode === "repository"
      ? zhLabel("仓库提交历史", "Repository commit history")
      : zhLabel("当前选择的提交历史", "Selection commit history");
  const selectedDiffFiles = Array.isArray(selectedDiff?.files) ? selectedDiff.files : [];

  searchPreviewPanel.innerHTML = `
    <div class="search-preview-section">
      <div class="search-preview-title">${escapeHtml(cleanDisplayText(repository.full_name || "", "GitHub repository"))}</div>
      <div class="search-preview-meta">${escapeHtml([
        activeRefKind === "commit"
          ? `${zhLabel("commit", "commit")} ${activeRef.slice(0, 7)}`
          : cleanDisplayText(repository.default_branch || ""),
        Number.isFinite(repository.stargazers_count) ? `★${repository.stargazers_count}` : "",
        cleanDisplayText(repository.language || ""),
        cleanDisplayText(preview.target_kind || "", ""),
      ].filter(Boolean).join(" / "))}</div>
      <div class="search-preview-snippet">${escapeHtml(cleanDisplayText(repository.description || "", ""))}</div>
      <div class="search-preview-breadcrumb">${escapeHtml(zhLabel("当前位置", "Current path"))}: ${escapeHtml(pathLabel)}</div>
      <div class="search-preview-action-row">
        <button type="button" class="git-inline-action" data-search-github-history="back" ${githubPreviewCanGoBack() ? "" : "disabled"}>
          ${escapeHtml(zhLabel("返回", "Back"))}
        </button>
        <button type="button" class="git-inline-action" data-search-github-history="forward" ${githubPreviewCanGoForward() ? "" : "disabled"}>
          ${escapeHtml(zhLabel("前进", "Forward"))}
        </button>
        <button type="button" class="git-inline-action" data-search-open="${escapeHtml(repoUrl)}" ${repoUrl ? "" : "disabled"}>
          ${escapeHtml(zhLabel("打开仓库", "Open repo"))}
        </button>
        <button
          type="button"
          class="git-inline-action"
          data-search-github-root="${escapeHtml(cleanDisplayText(repository.full_name || "", ""))}"
          data-search-github-branch="${escapeHtml(branch)}"
          data-search-github-commit-sha="${escapeHtml(activeCommitContextSha || "")}"
        >
          ${escapeHtml(zhLabel("根目录", "Root"))}
        </button>
        <button
          type="button"
          class="git-inline-action${historyScopeMode === "repository" ? " is-active" : ""}"
          data-search-github-history-scope="repository"
          data-search-github-repo="${escapeHtml(cleanDisplayText(repository.full_name || "", ""))}"
          data-search-github-branch="${escapeHtml(branch)}"
          data-search-github-path="${escapeHtml(cleanDisplayText(selectedFile?.path || preview.path || "", ""))}"
          data-search-github-commit-sha="${escapeHtml(activeCommitContextSha || "")}"
        >
          ${escapeHtml(zhLabel("仓库提交", "Repo commits"))}
        </button>
        <button
          type="button"
          class="git-inline-action${historyScopeMode !== "repository" ? " is-active" : ""}"
          data-search-github-history-scope="selection"
          data-search-github-repo="${escapeHtml(cleanDisplayText(repository.full_name || "", ""))}"
          data-search-github-branch="${escapeHtml(branch)}"
          data-search-github-path="${escapeHtml(cleanDisplayText(selectedFile?.path || preview.path || "", ""))}"
          data-search-github-commit-sha="${escapeHtml(activeCommitContextSha || "")}"
        >
          ${escapeHtml(zhLabel("当前选择提交", "Selection commits"))}
        </button>
        ${
          parentPath || selectedFile
            ? `
              <button
                type="button"
                class="git-inline-action"
                data-search-github-path="${escapeHtml(parentPath)}"
                data-search-github-repo="${escapeHtml(cleanDisplayText(repository.full_name || "", ""))}"
                data-search-github-branch="${escapeHtml(branch)}"
                data-search-github-commit-sha="${escapeHtml(activeCommitContextSha || "")}"
              >
                ${escapeHtml(zhLabel("上一级", "Up"))}
              </button>
            `
            : ""
        }
      </div>
      ${historyEntry ? `<div class="search-preview-meta">${escapeHtml(zhLabel("预览历史", "Preview history"))}: ${escapeHtml(githubPreviewHistoryLabel(historyEntry))}</div>` : ""}
    </div>
    ${
      selectedFile && cleanDisplayText(selectedFile.path || "", "")
        ? `
          <div class="search-preview-section">
            <div class="search-preview-title">${escapeHtml(cleanDisplayText(selectedFile.path || "", "Selected file"))}</div>
            <div class="search-preview-meta">${escapeHtml([
              selectedFileLanguage,
              `${Number(selectedFile?.size || 0)} B`,
              activeCommitContextSha ? `${zhLabel("版本", "Version")} ${activeCommitContextSha.slice(0, 7)}` : "",
            ].filter(Boolean).join(" / "))}</div>
            <div class="search-preview-action-row">
              <button type="button" class="git-inline-action" data-search-open="${escapeHtml(selectedFileHtml)}" ${selectedFileHtml ? "" : "disabled"}>
                ${escapeHtml(zhLabel("打开文件页", "Open blob"))}
              </button>
              <button type="button" class="git-inline-action" data-search-open="${escapeHtml(selectedFileRaw)}" ${selectedFileRaw ? "" : "disabled"}>
                ${escapeHtml(zhLabel("打开原始内容", "Open raw"))}
              </button>
            </div>
            <div class="search-preview-code-block markdown-body">${renderHighlightedCodeBlock(selectedFileContent, selectedFileLanguage, "search-preview-code")}</div>
          </div>
        `
        : ""
    }
    ${
      commitList.length
        ? `
          <div class="search-preview-section">
            <div class="search-preview-title">${escapeHtml(historyTitle)}</div>
            <div class="search-preview-meta">${escapeHtml(
              historyScopeMode === "repository"
                ? zhLabel("仓库根目录", "Repository root")
                : (historyPath || zhLabel("仓库根目录", "Repository root"))
            )}</div>
            <div class="git-commit-list search-preview-commit-list">
              ${commitList.map((commit) => {
                const commitSha = cleanDisplayText(commit?.sha || "", "");
                const selected = commitSha && commitSha === selectedCommitSha;
                const isCompareBase = commitSha && commitSha === compareBaseSha;
                const isCompareHead = commitSha && commitSha === compareHeadSha;
                return `
                  <div class="search-preview-commit-shell">
                    <button
                      type="button"
                      class="git-commit-item search-preview-commit-item${selected ? " is-selected" : ""}${isCompareBase ? " is-compare-base" : ""}${isCompareHead ? " is-compare-head" : ""}"
                      data-search-github-commit="${escapeHtml(commitSha)}"
                      data-search-github-repo="${escapeHtml(cleanDisplayText(repository.full_name || "", ""))}"
                      data-search-github-branch="${escapeHtml(branch)}"
                      data-search-github-path="${escapeHtml(historyPath)}"
                    >
                      <div class="git-commit-main">
                        <div class="git-commit-subject">${escapeHtml(cleanDisplayText(commit?.subject || "", zhLabel("提交", "Commit")))}</div>
                        <div class="git-commit-meta">${escapeHtml([
                          cleanDisplayText(commit?.short_sha || "", ""),
                          cleanDisplayText(commit?.date || "", ""),
                        ].filter(Boolean).join(" / "))}</div>
                      </div>
                      <div class="git-commit-author">${escapeHtml(cleanDisplayText(commit?.author || "", ""))}</div>
                    </button>
                    <div class="search-preview-commit-actions">
                      <button type="button" class="git-inline-action${isCompareBase ? " is-active" : ""}" data-search-github-compare-base="${escapeHtml(commitSha)}">${escapeHtml(zhLabel("Base", "Base"))}</button>
                      <button type="button" class="git-inline-action${isCompareHead ? " is-active" : ""}" data-search-github-compare-head="${escapeHtml(commitSha)}">${escapeHtml(zhLabel("Head", "Head"))}</button>
                    </div>
                  </div>
                `;
              }).join("")}
            </div>
          </div>
        `
        : ""
    }
    ${
      commitList.length
        ? `
          <div class="search-preview-section search-preview-compare-shell">
            <div class="search-preview-compare-head">
              <div>
                <div class="search-preview-title">${escapeHtml(zhLabel("Commit Compare", "Commit Compare"))}</div>
                <div class="search-preview-meta">${escapeHtml(
                  compareBaseSha && compareHeadSha
                    ? `${compareBaseSha.slice(0, 7)} .. ${compareHeadSha.slice(0, 7)}`
                    : zhLabel("从历史列表中选择 Base 和 Head 来比较两个提交。", "Choose Base / Head from the history list to compare any two commits."),
                )}</div>
              </div>
              <div class="search-preview-action-row">
                <button
                  type="button"
                  class="git-inline-action"
                  data-search-open="${escapeHtml(cleanDisplayText(compare?.html_url || compare?.permalink_url || "", ""))}"
                  ${cleanDisplayText(compare?.html_url || compare?.permalink_url || "", "") ? "" : "disabled"}
                >${escapeHtml(zhLabel("打开对比页", "Open compare"))}</button>
                <button
                  type="button"
                  class="git-inline-action"
                  data-search-github-clear-compare="true"
                  ${compareBaseSha || compareHeadSha ? "" : "disabled"}
                >${escapeHtml(zhLabel("清除对比", "Clear compare"))}</button>
              </div>
            </div>
            ${
              compareBaseSha && compareHeadSha
                ? compare?.available === true
                  ? `
                    <div class="search-preview-meta">${escapeHtml([
                      compare?.status ? `${zhLabel("状态", "Status")}: ${cleanDisplayText(compare.status, "")}` : "",
                      Number.isFinite(compare?.ahead_by) ? `${zhLabel("Ahead", "Ahead")} ${Number(compare.ahead_by || 0)}` : "",
                      Number.isFinite(compare?.behind_by) ? `${zhLabel("Behind", "Behind")} ${Number(compare.behind_by || 0)}` : "",
                      Number.isFinite(compare?.file_count) ? zhLabel(`${Number(compare.file_count || 0)} 个文件`, `${Number(compare.file_count || 0)} files`) : "",
                    ].filter(Boolean).join(" / "))}</div>
                    <div class="search-preview-compare-file-list">
                      ${compareFiles.map((file) => `
                        <section class="search-preview-compare-file">
                          <div class="search-preview-compare-file-head">
                            <div class="search-preview-title">${escapeHtml(cleanDisplayText(file?.path || "", "file"))}</div>
                            <div class="search-preview-meta">${escapeHtml([
                              cleanDisplayText(file?.status || "", ""),
                              Number.isFinite(file?.additions) || Number.isFinite(file?.deletions)
                                ? `+${Number(file?.additions || 0)} / -${Number(file?.deletions || 0)}`
                                : "",
                            ].filter(Boolean).join(" / "))}</div>
                          </div>
                          <div class="search-preview-diff-shell">${renderGitHubPreviewDiff(file, { sideBySide: true })}</div>
                        </section>
                      `).join("")}
                    </div>
                  `
                  : `<div class="search-preview-empty">${escapeHtml(cleanDisplayText(compare?.detail || "", zhLabel("当前选择没有可用的多文件对比 diff。", "No multi-file compare diff is available for the current selection.")))}</div>`
                : `<div class="search-preview-empty">${escapeHtml(zhLabel("请在历史列表中选择一个 Base 和一个 Head 提交。", "Pick one commit as Base and another as Head from the history list."))}</div>`
            }
          </div>
        `
        : ""
    }
    ${
      selectedCommitSha
        ? `
          <div class="search-preview-section">
            <div class="search-preview-title">${escapeHtml(
              selectedFile && cleanDisplayText(selectedFile.path || "", "")
                ? zhLabel("提交 diff", "Commit diff")
                : zhLabel("仓库提交 diff", "Repository commit diff"),
            )}</div>
            <div class="search-preview-meta">${escapeHtml([
              cleanDisplayText(selectedCommit?.short_sha || selectedCommitSha.slice(0, 7), ""),
              cleanDisplayText(selectedCommit?.subject || "", ""),
              selectedDiff?.status ? `${zhLabel("状态", "Status")}: ${cleanDisplayText(selectedDiff.status, "")}` : "",
              Number.isFinite(selectedDiff?.additions) || Number.isFinite(selectedDiff?.deletions)
                ? `+${Number(selectedDiff?.additions || 0)} / -${Number(selectedDiff?.deletions || 0)}`
                : "",
              Number.isFinite(selectedDiff?.file_count) ? zhLabel(`${Number(selectedDiff.file_count || 0)} 个文件`, `${Number(selectedDiff.file_count || 0)} files`) : "",
            ].filter(Boolean).join(" / "))}</div>
            <div class="search-preview-action-row">
              <button type="button" class="git-inline-action" data-search-open="${escapeHtml(cleanDisplayText(selectedCommit?.html_url || "", ""))}" ${cleanDisplayText(selectedCommit?.html_url || "", "") ? "" : "disabled"}>
                ${escapeHtml(zhLabel("打开提交页", "Open commit"))}
              </button>
              <button type="button" class="git-inline-action" data-search-github-clear-commit="true">
                ${escapeHtml(zhLabel("回到分支头部", "Back to branch head"))}
              </button>
            </div>
            ${
              selectedFile && cleanDisplayText(selectedFile.path || "", "")
                ? `<div class="search-preview-diff-shell">${renderGitHubPreviewDiff(selectedDiff)}</div>`
                : selectedDiff?.available === true && selectedDiffFiles.length
                  ? `
                    <div class="search-preview-compare-file-list">
                      ${selectedDiffFiles.map((file) => `
                        <section class="search-preview-compare-file">
                          <div class="search-preview-compare-file-head">
                            <div class="search-preview-title">${escapeHtml(cleanDisplayText(file?.path || "", "file"))}</div>
                            <div class="search-preview-meta">${escapeHtml([
                              cleanDisplayText(file?.status || "", ""),
                              Number.isFinite(file?.additions) || Number.isFinite(file?.deletions)
                                ? `+${Number(file?.additions || 0)} / -${Number(file?.deletions || 0)}`
                                : "",
                            ].filter(Boolean).join(" / "))}</div>
                          </div>
                          <div class="search-preview-diff-shell">${renderGitHubPreviewDiff(file, { sideBySide: true })}</div>
                        </section>
                      `).join("")}
                    </div>
                  `
                  : `<div class="search-preview-empty">${escapeHtml(cleanDisplayText(selectedDiff?.detail || "", zhLabel("所选提交没有可用的仓库级 diff。", "No repository-wide diff is available for the selected commit.")))}</div>`
            }
          </div>
        `
        : ""
    }
    ${
      readme && readmeContent.trim()
        ? `
          <div class="search-preview-section">
            <div class="search-preview-title">${escapeHtml(cleanDisplayText(readme.path || "README", "README"))}</div>
            <div class="search-preview-meta">${escapeHtml(readmeLanguage)}</div>
            <div class="search-preview-action-row">
              <button type="button" class="git-inline-action" data-search-open="${escapeHtml(readmeHtml)}" ${readmeHtml ? "" : "disabled"}>
                ${escapeHtml(zhLabel("打开 README", "Open README"))}
              </button>
              <button type="button" class="git-inline-action" data-search-open="${escapeHtml(readmeRaw)}" ${readmeRaw ? "" : "disabled"}>
                ${escapeHtml(zhLabel("打开原始内容", "Open raw"))}
              </button>
            </div>
            <div class="search-preview-code-block markdown-body">${renderHighlightedCodeBlock(readmeContent, readmeLanguage, "search-preview-code")}</div>
          </div>
        `
        : ""
    }
    ${
      textMatches.length
        ? `
          <div class="search-preview-section">
            <div class="search-preview-title">${escapeHtml(zhLabel("命中片段", "Match snippets"))}</div>
            <div class="search-preview-hit-list">
              ${textMatches.map((snippet) => `
                <pre class="search-preview-hit">${escapeHtml(snippet)}</pre>
              `).join("")}
            </div>
          </div>
        `
        : ""
    }
    <div class="search-preview-section">
      <div class="search-preview-title">${escapeHtml(zhLabel("仓库目录树", "Repository tree"))}</div>
      <div class="search-preview-entry-list">
        ${entries.length
          ? entries.map((entry) => `
            <div class="search-preview-entry-shell">
              <button
                type="button"
                class="search-preview-entry search-preview-entry-button"
                data-search-github-path="${escapeHtml(cleanDisplayText(entry?.path || "", ""))}"
                data-search-github-repo="${escapeHtml(cleanDisplayText(repository.full_name || "", ""))}"
                data-search-github-branch="${escapeHtml(branch)}"
                data-search-github-commit-sha="${escapeHtml(activeCommitContextSha || "")}"
                data-search-github-kind="${escapeHtml(cleanDisplayText(entry?.kind || "", ""))}"
              >
                <div class="search-preview-entry-head">
                  <div class="search-preview-title">${escapeHtml(cleanDisplayText(entry?.path || entry?.name || "", "entry"))}</div>
                  <div class="search-preview-entry-kind">${escapeHtml(cleanDisplayText(entry?.kind || "", ""))}</div>
                </div>
                <div class="search-preview-entry-main">${escapeHtml(cleanDisplayText(entry?.html_url || entry?.download_url || "", ""))}</div>
              </button>
              <div class="search-preview-entry-actions">
                <button type="button" class="git-inline-action" data-search-open="${escapeHtml(cleanDisplayText(entry?.html_url || "", ""))}" ${cleanDisplayText(entry?.html_url || "", "") ? "" : "disabled"}>
                  ${escapeHtml(cleanDisplayText(entry?.kind || "", "") === "dir" ? zhLabel("打开目录", "Open dir") : zhLabel("打开文件页", "Open blob"))}
                </button>
                <button type="button" class="git-inline-action" data-search-open="${escapeHtml(cleanDisplayText(entry?.download_url || "", ""))}" ${cleanDisplayText(entry?.download_url || "", "") ? "" : "disabled"}>
                  ${escapeHtml(zhLabel("打开原始内容", "Open raw"))}
                </button>
              </div>
            </div>
          `).join("")
          : `<div class="search-preview-empty">${escapeHtml(zhLabel("当前预览路径下没有可显示的条目。", "No entries available for the selected preview path."))}</div>`}
      </div>
    </div>
  `;

  searchPreviewPanel.querySelectorAll("[data-search-open]").forEach((button) => {
    button.addEventListener("click", async () => {
      const href = cleanDisplayText(button.getAttribute("data-search-open") || "", "");
      if (!href) {
        showToast(t("searchNoUrl"));
        return;
      }
      try {
        await openUrlInAppBrowser(href);
      } catch (error) {
        console.error(error);
        showToast(cleanDisplayText(error?.message || "", "") || t("toastSendFailed"));
      }
    });
  });

  searchPreviewPanel.querySelectorAll("[data-search-github-root]").forEach((button) => {
    button.addEventListener("click", async () => {
      const repoFullName = cleanDisplayText(button.getAttribute("data-search-github-root") || "", "");
      const buttonBranch = cleanDisplayText(button.getAttribute("data-search-github-branch") || "", "");
      const commitSha = normalizeGitHubPreviewCommit(button.getAttribute("data-search-github-commit-sha") || "");
      if (!repoFullName) return;
        await loadGitHubPreview(repoFullName, buttonBranch || null, null, {
          sourceIndex: searchState.githubPreviewSourceIndex,
          resetSourceIndex: false,
          commitSha,
          compareBaseSha,
          compareHeadSha,
          historyScopeMode,
        });
      });
    });

  searchPreviewPanel.querySelectorAll("[data-search-github-path]").forEach((button) => {
    button.addEventListener("click", async () => {
      const repoFullName = cleanDisplayText(button.getAttribute("data-search-github-repo") || "", "");
      const buttonBranch = cleanDisplayText(button.getAttribute("data-search-github-branch") || "", "");
      const path = cleanDisplayText(button.getAttribute("data-search-github-path") || "", "");
      const commitSha = normalizeGitHubPreviewCommit(button.getAttribute("data-search-github-commit-sha") || "");
      if (!repoFullName) return;
        await loadGitHubPreview(repoFullName, buttonBranch || null, path || null, {
          sourceIndex: searchState.githubPreviewSourceIndex,
          resetSourceIndex: false,
          commitSha,
          compareBaseSha,
          compareHeadSha,
          historyScopeMode,
        });
      });
    });

  searchPreviewPanel.querySelectorAll("[data-search-github-commit]").forEach((button) => {
    button.addEventListener("click", async () => {
      const repoFullName = cleanDisplayText(button.getAttribute("data-search-github-repo") || "", "");
      const buttonBranch = cleanDisplayText(button.getAttribute("data-search-github-branch") || "", "");
      const path = cleanDisplayText(button.getAttribute("data-search-github-path") || "", "");
      const commitSha = normalizeGitHubPreviewCommit(button.getAttribute("data-search-github-commit") || "");
      if (!repoFullName || !commitSha) return;
        await loadGitHubPreview(repoFullName, buttonBranch || null, path || null, {
          sourceIndex: searchState.githubPreviewSourceIndex,
          resetSourceIndex: false,
          commitSha,
          compareBaseSha,
          compareHeadSha,
          historyScopeMode,
        });
      });
    });

  searchPreviewPanel.querySelectorAll("[data-search-github-compare-base]").forEach((button) => {
    button.addEventListener("click", async () => {
      const nextBaseSha = normalizeGitHubPreviewCommit(button.getAttribute("data-search-github-compare-base") || "");
      if (!nextBaseSha) return;
      let nextHeadSha = normalizeGitHubPreviewCommit(compareHeadSha || selectedCommitSha);
      if (!nextHeadSha || nextHeadSha === nextBaseSha) {
        nextHeadSha = commitList
          .map((entry) => normalizeGitHubPreviewCommit(entry?.sha || ""))
          .find((value) => value && value !== nextBaseSha) || "";
      }
      if (!nextHeadSha || nextHeadSha === nextBaseSha) return;
      await loadGitHubPreview(cleanDisplayText(repository.full_name || "", ""), branch || null, cleanDisplayText(selectedFile?.path || preview.path || "", "") || null, {
        sourceIndex: searchState.githubPreviewSourceIndex,
        resetSourceIndex: false,
        commitSha: nextHeadSha,
        compareBaseSha: nextBaseSha,
        compareHeadSha: nextHeadSha,
        historyScopeMode,
      });
    });
  });

  searchPreviewPanel.querySelectorAll("[data-search-github-compare-head]").forEach((button) => {
    button.addEventListener("click", async () => {
      const nextHeadSha = normalizeGitHubPreviewCommit(button.getAttribute("data-search-github-compare-head") || "");
      if (!nextHeadSha) return;
      let nextBaseSha = normalizeGitHubPreviewCommit(compareBaseSha);
      if (!nextBaseSha || nextBaseSha === nextHeadSha) {
        nextBaseSha = commitList
          .map((entry) => normalizeGitHubPreviewCommit(entry?.sha || ""))
          .find((value) => value && value !== nextHeadSha) || "";
      }
      if (!nextBaseSha || nextBaseSha === nextHeadSha) return;
      await loadGitHubPreview(cleanDisplayText(repository.full_name || "", ""), branch || null, cleanDisplayText(selectedFile?.path || preview.path || "", "") || null, {
        sourceIndex: searchState.githubPreviewSourceIndex,
        resetSourceIndex: false,
        commitSha: nextHeadSha,
        compareBaseSha: nextBaseSha,
        compareHeadSha: nextHeadSha,
        historyScopeMode,
      });
    });
  });

  searchPreviewPanel.querySelectorAll("[data-search-github-clear-compare]").forEach((button) => {
    button.addEventListener("click", async () => {
      const repoFullName = cleanDisplayText(repository.full_name || "", "");
      if (!repoFullName) return;
      await loadGitHubPreview(repoFullName, branch || null, cleanDisplayText(selectedFile?.path || preview.path || "", "") || null, {
        sourceIndex: searchState.githubPreviewSourceIndex,
        resetSourceIndex: false,
        commitSha: selectedCommitSha || null,
        compareBaseSha: null,
        compareHeadSha: null,
        historyScopeMode,
      });
    });
  });

  searchPreviewPanel.querySelectorAll("[data-search-github-clear-commit]").forEach((button) => {
    button.addEventListener("click", async () => {
      const repoFullName = cleanDisplayText(repository.full_name || "", "");
      if (!repoFullName) return;
      await loadGitHubPreview(repoFullName, branch || null, cleanDisplayText(selectedFile?.path || preview.path || "", "") || null, {
        sourceIndex: searchState.githubPreviewSourceIndex,
        resetSourceIndex: false,
        commitSha: null,
        compareBaseSha,
        compareHeadSha,
        historyScopeMode,
      });
    });
  });

  searchPreviewPanel.querySelectorAll("[data-search-github-history-scope]").forEach((button) => {
    button.addEventListener("click", async () => {
      const repoFullName = cleanDisplayText(button.getAttribute("data-search-github-repo") || "", "");
      const buttonBranch = cleanDisplayText(button.getAttribute("data-search-github-branch") || "", "");
      const path = cleanDisplayText(button.getAttribute("data-search-github-path") || "", "");
      const commitSha = normalizeGitHubPreviewCommit(button.getAttribute("data-search-github-commit-sha") || "");
      const nextHistoryScopeMode = cleanDisplayText(button.getAttribute("data-search-github-history-scope") || "", "");
      if (!repoFullName || !nextHistoryScopeMode || nextHistoryScopeMode === historyScopeMode) return;
      await loadGitHubPreview(repoFullName, buttonBranch || null, path || null, {
        sourceIndex: searchState.githubPreviewSourceIndex,
        resetSourceIndex: false,
        commitSha,
        compareBaseSha,
        compareHeadSha,
        historyScopeMode: nextHistoryScopeMode,
      });
    });
  });

  searchPreviewPanel.querySelectorAll("[data-search-github-history]").forEach((button) => {
    button.addEventListener("click", async () => {
      const direction = cleanDisplayText(button.getAttribute("data-search-github-history") || "", "");
      const delta = direction === "back" ? -1 : direction === "forward" ? 1 : 0;
      if (!delta) return;
      const nextIndex = Number(searchState.githubPreviewHistoryIndex) + delta;
      if (nextIndex < 0 || nextIndex >= searchState.githubPreviewHistory.length) return;
      const entry = searchState.githubPreviewHistory[nextIndex];
      if (!entry) return;
      await loadGitHubPreview(entry.repoFullName, entry.branch || null, entry.path || null, {
        sourceIndex: searchState.githubPreviewSourceIndex,
        resetSourceIndex: false,
        commitSha: entry.commitSha || null,
        compareBaseSha: entry.compareBaseSha || null,
        compareHeadSha: entry.compareHeadSha || null,
        historyScopeMode: entry.historyScopeMode || null,
        pushHistory: false,
        historyIndex: nextIndex,
      });
    });
  });
}

function deriveSearchHref(mode, item) {
  if (!item || typeof item !== "object") return "";
  if (mode === "papers") {
    return cleanDisplayText(item?.urls?.landing_page || item?.urls?.pdf || item?.urls?.local_path || "");
  }
  if (mode === "tracking" || mode === "benchmarks") {
    return cleanDisplayText(item?.url || "");
  }
  if (mode === "datasets") {
    return cleanDisplayText(item?.url || item?.path || item?.source_url || "");
  }
  if (mode === "models") {
    return cleanDisplayText(item?.url || item?.download_url || item?.path || "");
  }
  if (mode === "github") {
    return cleanDisplayText(item?.html_url || item?.repository_url || item?.url || "");
  }
  return cleanDisplayText(item?.url || "");
}

function renderSearchHealthStrip() {
  if (!searchHealthStrip) return;
  const health = searchState.health || {};
  const entries = [
    ["searchWebHealth", health.web_search],
    ["searchPapersHealth", health.paper_search],
    ["searchTrackingHealth", health.tracking_search],
    ["searchBenchmarksHealth", health.benchmark_search],
    ["searchModelsHealth", health.models_search],
    ["searchDatasetHealth", health.dataset_entrypoint],
    ["searchGitHubHealth", health.github_search],
  ];
  searchHealthStrip.innerHTML = entries
    .map(([labelKey, item]) => {
      const status = cleanDisplayText(item?.status || "unknown", "unknown");
      const detail = cleanDisplayText(item?.detail || "");
      const providers = Array.isArray(item?.providers) ? item.providers : [];
      const providerMarkup = providers.length
        ? `
          <div class="search-health-provider-list">
            ${providers
              .map((provider) => {
                const providerStatus = cleanDisplayText(provider?.status || "unknown", "unknown");
                const providerDetail = cleanDisplayText(provider?.detail || "");
                const providerName = cleanDisplayText(provider?.name || "", "provider");
                return `
                  <div class="search-health-provider ${searchStatusClass(providerStatus)}">
                    <div class="search-health-provider-top">
                      <span class="search-health-provider-name">${escapeHtml(providerName)}</span>
                      <span class="search-health-provider-state">${escapeHtml(searchStatusLabel(providerStatus))}</span>
                    </div>
                    <div class="search-health-provider-detail">${escapeHtml(providerDetail || "-")}</div>
                  </div>
                `;
              })
              .join("")}
          </div>
        `
        : "";
      return `
        <article class="search-health-card ${searchStatusClass(status)}">
          <div class="search-health-card-top">
            <span class="search-health-name">${escapeHtml(t(labelKey))}</span>
            <span class="search-health-state">${escapeHtml(searchStatusLabel(status))}</span>
          </div>
          <div class="search-health-detail">${escapeHtml(detail || "-")}</div>
          ${providerMarkup}
        </article>
      `;
    })
    .join("");
}

function renderSearchResults() {
  if (!searchResults) return;
  const mode = searchMode;
  const payload = searchState.results;
  const items = normalizeSearchResults(mode, payload);
  if (searchState.loading) {
    searchResults.innerHTML = `<div class="git-empty">${escapeHtml(t("searchLoading"))}</div>`;
    return;
  }
  if (searchState.error) {
    searchResults.innerHTML = `
      <article class="search-result-card is-error">
        <div class="search-result-title">${escapeHtml(t("searchError"))}</div>
        <div class="search-result-snippet">${escapeHtml(searchState.error)}</div>
      </article>
    `;
    return;
  }
  if (!items.length) {
    const hints = normalizeSearchHints(payload);
    searchResults.innerHTML = `
      <article class="search-result-card is-error">
        <div class="search-result-title">${escapeHtml(t("searchEmpty"))}</div>
        ${
          hints.length
            ? `<div class="search-result-snippet">${escapeHtml(hints.join(" | "))}</div>`
            : ""
        }
      </article>
    `;
    return;
  }

  searchResults.innerHTML = items
    .map((item, index) => {
      if (mode === "papers") {
        const href = deriveSearchHref(mode, item);
        const authors = Array.isArray(item.authors) ? item.authors.filter(Boolean).slice(0, 4).join(", ") : "";
        const year = item?.year ? String(item.year) : "";
        const provider = cleanDisplayText(item?.provider || "");
        const snippet = cleanDisplayText(item?.abstract_text || item?.snippet || "");
        return `
          <article class="search-result-card">
            <div class="search-result-topline">
              <div class="search-result-title">${escapeHtml(cleanDisplayText(item?.title || "", "Untitled paper"))}</div>
              ${provider ? `<span class="search-result-badge">${escapeHtml(provider)}</span>` : ""}
            </div>
            <div class="search-result-meta">${escapeHtml([authors, year].filter(Boolean).join(" / "))}</div>
            <div class="search-result-snippet">${escapeHtml(snippet)}</div>
            <div class="search-result-actions">
              <button type="button" class="git-inline-action" data-search-open="${escapeHtml(href)}" ${href ? "" : "disabled"}>
                ${escapeHtml(t("searchOpen"))}
              </button>
            </div>
          </article>
        `;
      }

      if (mode === "tracking" || mode === "benchmarks") {
        const href = deriveSearchHref(mode, item);
        const provider = cleanDisplayText(item?.provider || "");
        const kind = cleanDisplayText(item?.kind || item?.benchmark_family || "");
        const rank = Number.isFinite(item?.rank) ? `#${item.rank}` : "";
        const snippet = cleanDisplayText(item?.snippet || href || "");
        return `
          <article class="search-result-card">
            <div class="search-result-topline">
              <div class="search-result-title">${escapeHtml(cleanDisplayText(item?.title || "", mode === "tracking" ? "Tracking result" : "Benchmark result"))}</div>
              ${provider ? `<span class="search-result-badge">${escapeHtml(provider)}</span>` : ""}
            </div>
            <div class="search-result-meta">${escapeHtml([kind, rank].filter(Boolean).join(" / "))}</div>
            <div class="search-result-snippet">${escapeHtml(snippet)}</div>
            <div class="search-result-actions">
              <button type="button" class="git-inline-action" data-search-open="${escapeHtml(href)}" ${href ? "" : "disabled"}>
                ${escapeHtml(t("searchOpen"))}
              </button>
            </div>
          </article>
        `;
      }

      if (mode === "datasets") {
        const href = deriveSearchHref(mode, item);
        const datasetId = cleanDisplayText(item?.dataset_id || "");
        const provider = cleanDisplayText(item?.provider || "");
        const taskHint = cleanDisplayText(item?.task_hint || "");
        const manifest = href ? searchState.manifests[href] : null;
        const manifestReady = manifest && manifest.status === "success";
        return `
          <article class="search-result-card">
            <div class="search-result-topline">
              <div class="search-result-title">${escapeHtml(cleanDisplayText(item?.title || datasetId || "Dataset"))}</div>
              ${provider ? `<span class="search-result-badge">${escapeHtml(provider)}</span>` : ""}
            </div>
            <div class="search-result-meta">${escapeHtml([datasetId, taskHint].filter(Boolean).join(" / "))}</div>
            <div class="search-result-snippet">${escapeHtml(href)}</div>
            ${
              manifestReady
                ? `<pre class="search-manifest-block">${escapeHtml(JSON.stringify(manifest, null, 2))}</pre>`
                : ""
            }
            <div class="search-result-actions">
              <button type="button" class="git-inline-action" data-search-open="${escapeHtml(href)}" ${href ? "" : "disabled"}>
                ${escapeHtml(t("searchOpen"))}
              </button>
              <button
                type="button"
                class="git-inline-action"
                data-search-dataset-manifest="${escapeHtml(String(index))}"
                ${href ? "" : "disabled"}
              >
                ${escapeHtml(manifestReady ? t("searchManifestReady") : t("searchUseDataset"))}
              </button>
            </div>
          </article>
        `;
      }

      if (mode === "models") {
        const href = deriveSearchHref(mode, item);
        const badge = cleanDisplayText(item?.provider || "", "onnx-model-zoo");
        const meta = [
          cleanDisplayText(item?.category || ""),
          cleanDisplayText(item?.family || ""),
          item?.opset_version ? `opset ${item.opset_version}` : "",
        ].filter(Boolean).join(" / ");
        const tags = Array.isArray(item?.tags) ? item.tags.filter(Boolean).slice(0, 4).join(", ") : "";
        const snippet = cleanDisplayText(item?.snippet || item?.path || "");
        const secondary = [tags, cleanDisplayText(item?.path || "")].filter(Boolean).join(" / ");
        return `
          <article class="search-result-card">
            <div class="search-result-topline">
              <div class="search-result-title">${escapeHtml(cleanDisplayText(item?.title || item?.model_name || "", "ONNX model"))}</div>
              ${badge ? `<span class="search-result-badge">${escapeHtml(badge)}</span>` : ""}
            </div>
            <div class="search-result-meta">${escapeHtml(meta || secondary || href)}</div>
            <div class="search-result-snippet">${escapeHtml(snippet)}</div>
            ${secondary && secondary !== meta ? `<div class="search-result-meta">${escapeHtml(secondary)}</div>` : ""}
            <div class="search-result-actions">
              <button type="button" class="git-inline-action" data-search-open="${escapeHtml(href)}" ${href ? "" : "disabled"}>
                ${escapeHtml(t("searchOpen"))}
              </button>
            </div>
          </article>
        `;
      }

      if (mode === "github") {
        const href = deriveSearchHref("github", item);
        const selectionKey = githubPreviewSelectionKey(item);
        const isSelected = (selectionKey && selectionKey === cleanDisplayText(searchState.githubPreview?.selection_key || "", ""))
          || index === Number(searchState.githubPreviewSourceIndex ?? -1);
        const title = cleanDisplayText(
          item?.full_name || item?.repository_full_name || item?.title || "",
          "GitHub result",
        );
        const badge = cleanDisplayText(item?.language || item?.match_reason || "");
        const meta = [
          cleanDisplayText(item?.path || ""),
          cleanDisplayText(item?.default_branch || ""),
          Number.isFinite(item?.stargazers_count) ? `★${item.stargazers_count}` : "",
        ].filter(Boolean).join(" / ");
        const snippet = cleanDisplayText(
          item?.repository_description || item?.description || payload?.detail || "",
        );
        return `
          <article class="search-result-card${isSelected ? " is-selected" : ""}">
            <div class="search-result-topline">
              <div class="search-result-title">${escapeHtml(title)}</div>
              ${badge ? `<span class="search-result-badge">${escapeHtml(badge)}</span>` : ""}
            </div>
            <div class="search-result-meta">${escapeHtml(meta || href)}</div>
            <div class="search-result-snippet">${escapeHtml(snippet)}</div>
            <div class="search-result-actions">
              <button
                type="button"
                class="git-inline-action"
                data-search-github-preview="${escapeHtml(String(index))}"
              >
                ${escapeHtml(zhLabel("预览", "Preview"))}
              </button>
              <button type="button" class="git-inline-action" data-search-open="${escapeHtml(href)}" ${href ? "" : "disabled"}>
                ${escapeHtml(t("searchOpen"))}
              </button>
            </div>
          </article>
        `;
      }

      const href = deriveSearchHref(mode, item);
      const engine = cleanDisplayText(item?.engine || "");
      return `
        <article class="search-result-card">
          <div class="search-result-topline">
            <div class="search-result-title">${escapeHtml(cleanDisplayText(item?.title || "", "Untitled result"))}</div>
            ${engine ? `<span class="search-result-badge">${escapeHtml(engine)}</span>` : ""}
          </div>
          <div class="search-result-meta">${escapeHtml(href)}</div>
          <div class="search-result-snippet">${escapeHtml(cleanDisplayText(item?.snippet || ""))}</div>
          <div class="search-result-actions">
            <button type="button" class="git-inline-action" data-search-open="${escapeHtml(href)}" ${href ? "" : "disabled"}>
              ${escapeHtml(t("searchOpen"))}
            </button>
          </div>
        </article>
      `;
    })
    .join("");

  searchResults.querySelectorAll("[data-search-open]").forEach((button) => {
    button.addEventListener("click", async () => {
      const href = cleanDisplayText(button.getAttribute("data-search-open") || "");
      if (!href) {
        showToast(t("searchNoUrl"));
        return;
      }
      try {
        await openUrlInAppBrowser(href);
      } catch (error) {
        console.error(error);
        showToast(cleanDisplayText(error?.message || "") || t("toastSendFailed"));
      }
    });
  });

  searchResults.querySelectorAll("[data-search-dataset-manifest]").forEach((button) => {
    button.addEventListener("click", async () => {
      const index = Number(button.getAttribute("data-search-dataset-manifest") || "-1");
      if (!Number.isFinite(index) || index < 0) return;
      try {
        await buildDatasetManifest(index);
      } catch (error) {
        console.error(error);
        showToast(appErrorMessage(error, "search", "searchError"));
      }
    });
  });

  searchResults.querySelectorAll("[data-search-github-preview]").forEach((button) => {
    button.addEventListener("click", async () => {
      const index = Number(button.getAttribute("data-search-github-preview") || "-1");
      if (!Number.isFinite(index) || index < 0) return;
      const githubItems = normalizeSearchResults("github", searchState.results);
      const item = githubItems[index];
      if (!item) return;
      const repoFullName = cleanDisplayText(item?.repository_full_name || item?.full_name || "", "");
      const path = cleanDisplayText(item?.path || "", "");
      if (!repoFullName) return;
      searchState.githubPreviewSourceIndex = index;
      await loadGitHubPreview(
        repoFullName,
        cleanDisplayText(item?.default_branch || "", "") || null,
        path || null,
        { sourceIndex: index },
      );
    });
  });
}

function renderSearchPanel() {
  if (searchQueryInput) {
    searchQueryInput.placeholder = searchPlaceholderForMode(searchMode);
  }
  if (searchModeSwitch) {
    setSegmentedValue(searchModeSwitch, searchMode);
  }
  if (searchRunButton) {
    searchRunButton.disabled = searchState.loading;
    searchRunButton.textContent = searchState.loading ? t("searchLoading") : t("searchRun");
  }
  const searchWorkspace = document.getElementById("search-workspace");
  if (searchWorkspace) {
    searchWorkspace.classList.toggle("is-github-preview", isSearchGitHubPreviewActive());
  }
  activityFlyout?.classList.toggle("is-search-github-preview", isSearchGitHubPreviewActive());
  appShell?.classList.toggle("has-search-github-preview", isSearchGitHubPreviewActive());
  syncShellLayoutVars();
  renderSearchHealthStrip();
  renderSearchResults();
  renderSearchPreviewPanel();
}

async function loadSearchHealth({ force = false } = {}) {
  if (searchState.health && !force) {
    renderSearchPanel();
    return searchState.health;
  }
  const response = await hostClient.search.health();
  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(errorText || `search health failed: ${response.status}`);
  }
  const payload = await response.json();
  searchState.health = payload?.data || null;
  renderSearchPanel();
  return searchState.health;
}

async function runSearch({ query = null, mode = searchMode, force = false } = {}) {
  const resolvedMode = normalizeChoice(mode, ["web", "papers", "tracking", "benchmarks", "models", "datasets", "github"], "web");
  const nextQuery = String(query ?? searchQueryInput?.value ?? searchState.lastQuery ?? "").trim();
  searchMode = resolvedMode;
  if (searchQueryInput && nextQuery !== searchQueryInput.value) {
    searchQueryInput.value = nextQuery;
  }
  if (!nextQuery) {
    searchState.error = currentLanguage === "zh" ? "请输入搜索词。" : "Enter a search query.";
    searchState.results = null;
    renderSearchPanel();
    return;
  }

  if (resolvedMode === "datasets" || resolvedMode === "github" || resolvedMode === "models" || resolvedMode === "tracking" || resolvedMode === "benchmarks") {
    await loadSearchHealth({ force });
  }

  searchState.loading = true;
  searchState.error = "";
  searchState.lastQuery = nextQuery;
  if (resolvedMode !== "datasets") {
    searchState.activeManifestUrl = "";
  }
  if (resolvedMode === "github") {
    searchState.githubPreview = null;
    searchState.githubPreviewLoading = false;
    searchState.githubPreviewError = "";
    searchState.githubPreviewSourceIndex = -1;
  } else {
    searchState.githubPreview = null;
    searchState.githubPreviewLoading = false;
    searchState.githubPreviewError = "";
    searchState.githubPreviewSourceIndex = -1;
  }
  renderSearchPanel();

  try {
    let response;
    if (resolvedMode === "papers") {
      response = await hostClient.search.papers(nextQuery, "auto", 8);
    } else if (resolvedMode === "tracking") {
      response = await hostClient.search.tracking(nextQuery, "auto", 8);
    } else if (resolvedMode === "benchmarks") {
      response = await hostClient.search.benchmarks(nextQuery, "mlperf", 8);
    } else if (resolvedMode === "models") {
      response = await hostClient.search.models(nextQuery, 8);
    } else if (resolvedMode === "github") {
      const githubMode = /\b(path:|filename:|extension:|language:|repo:)\b/i.test(nextQuery) ? "code" : "repositories";
      response = await hostClient.search.github(nextQuery, githubMode, 8);
    } else if (resolvedMode === "datasets") {
      response = await hostClient.search.datasets(nextQuery, 8);
    } else {
      response = await hostClient.search.web(nextQuery, 8);
    }
    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(errorText || `search failed: ${response.status}`);
    }
    const payload = await response.json();
    searchState.results = payload?.data || null;
    searchState.error = "";
    renderSearchPanel();
    showToast(t("toastSearchCopied"));
  } catch (error) {
    searchState.results = null;
    searchState.error = cleanDisplayText(error?.message || "") || appErrorMessage(error, "search", "searchError");
    renderSearchPanel();
    throw error;
  } finally {
    searchState.loading = false;
    renderSearchPanel();
  }
}

async function buildDatasetManifest(index) {
  const items = normalizeSearchResults("datasets", searchState.results);
  const item = items[index];
  const datasetUrl = deriveSearchHref("datasets", item);
  if (!datasetUrl) {
    showToast(t("searchNoUrl"));
    return;
  }
  const title = cleanDisplayText(item?.title || item?.dataset_id || "");
  const response = await hostClient.search.datasetManifest(datasetUrl, title || null);
  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(errorText || `dataset manifest failed: ${response.status}`);
  }
  const payload = await response.json();
  searchState.manifests = {
    ...searchState.manifests,
    [datasetUrl]: payload?.data || null,
  };
  searchState.activeManifestUrl = datasetUrl;
  renderSearchPanel();
}

function renderRunDebugLegacy(state) {
  if (!runDebugList || !runDebugSession) return;
  const configs = Array.isArray(state?.configs) ? state.configs : [];
  const active = state?.active || null;
  const activeFilePath = String(currentWorkspaceFile?.path || "");

  runDebugList.innerHTML = configs.length
    ? configs
        .map(
          (config) => `
            <button
              class="run-card${activeFilePath && config.file_hint && activeFilePath.endsWith(String(config.file_hint)) ? " is-contextual" : ""}${config.available === false ? " is-missing" : ""}"
              type="button"
              data-run-config-id="${escapeHtml(config.id)}"
              ${config.file_hint ? `data-open-workspace-file="${escapeHtml(config.file_hint)}"` : ""}
            >
              <span class="run-card-title">${escapeHtml(config.title || "")}</span>
              <span class="run-card-meta">${escapeHtml(config.command || "")}</span>
              ${
                config.available === false
                  ? `<span class="run-card-task run-card-dependency">${escapeHtml(formatRunDependencyMessage(config))}</span>`
                  : ""
              }
              <span class="extension-card-desc">${escapeHtml(config.category || "")}${config.file_hint ? ` · ${escapeHtml(config.file_hint)}` : ""}</span>
              <span class="run-card-task">${escapeHtml(config.task_type || "launch")}${config.detail ? ` · ${escapeHtml(config.detail || "")}` : ""}</span>
            </button>
          `,
        )
        .join("")
    : `<div class="git-empty">${escapeHtml(currentLanguage === "zh" ? "未检测到可运行配置。" : "No runnable configuration detected.")}</div>`;

  runDebugSession.hidden = !active;
  runDebugSession.innerHTML = active
    ? `
      <div class="run-session-head">
        <strong>${escapeHtml(active.title || "")}</strong>
        <button class="git-inline-action is-danger" type="button" data-run-debug-stop="true">${escapeHtml(currentLanguage === "zh" ? "停止" : "Stop")}</button>
      </div>
      <div class="run-session-meta">PID ${escapeHtml(String(active.pid || ""))}${active.started_at ? ` · ${escapeHtml(active.started_at || "")}` : ""}</div>
      <div class="run-session-meta">${escapeHtml(active.command || "")}</div>
      <button class="run-session-cwd" type="button" data-open-workspace-file=".">${escapeHtml(active.cwd || "")}</button>
      <div class="run-session-log">
        <div class="run-session-log-title">stdout</div>
        <pre>${escapeHtml(active.stdout_tail || "")}</pre>
      </div>
      <div class="run-session-log">
        <div class="run-session-log-title">stderr</div>
        <pre>${escapeHtml(active.stderr_tail || "")}</pre>
      </div>
    `
    : "";

  runDebugList.querySelectorAll("[data-run-config-id]").forEach((button) => {
    button.addEventListener("click", async () => {
      const configId = button.getAttribute("data-run-config-id") || "";
      if (!configId) return;
      runWorkspaceConfigAction(configId);
    });
  });
  bindTurnInteractionHandlers(runDebugList);
  bindTurnInteractionHandlers(runDebugSession);

  runDebugSession.querySelectorAll("[data-run-debug-stop]").forEach((button) => {
    button.addEventListener("click", async () => {
      try {
        await runRunDebugAction("stop");
      } catch (error) {
        console.error(error);
        showToast(appErrorMessage(error, "workspace", "toastSendFailed"));
      }
    });
  });
}

function renderRunDebug(state) {
  if (!runDebugList || !runDebugSession) return;
  const configs = Array.isArray(state?.configs) ? state.configs : [];
  const active = state?.active || null;
  const activeFilePath = String(currentWorkspaceFile?.path || "");
  const groupedConfigs = configs.reduce((groups, config) => {
    const category = String(config.category || "Other");
    if (!groups.has(category)) groups.set(category, []);
    groups.get(category).push(config);
    return groups;
  }, new Map());

  runDebugList.innerHTML = configs.length
    ? Array.from(groupedConfigs.entries())
        .map(([category, items]) => {
          const missingCount = items.filter((config) => config.available === false).length;
          return `
            <section class="run-config-group">
              <div class="run-config-group-head">
                <span>${escapeHtml(category)}</span>
                <span>${escapeHtml(String(items.length))}${missingCount ? ` / ${escapeHtml(String(missingCount))} missing` : ""}</span>
              </div>
              <div class="run-config-group-list">
                ${items
                  .map(
                    (config) => `
                      <article class="run-config-entry">
                        <button
                          class="run-card${activeFilePath && config.file_hint && activeFilePath.endsWith(String(config.file_hint)) ? " is-contextual" : ""}${config.available === false ? " is-missing" : ""}"
                          type="button"
                          data-run-config-id="${escapeHtml(config.id)}"
                          title="${escapeHtml(config.command || "")}"
                        >
                          <span class="run-card-topline">
                            <span class="run-card-title">${escapeHtml(config.title || "")}</span>
                            <span class="run-card-kind">${escapeHtml(config.task_type || "launch")}</span>
                          </span>
                          <span class="run-card-meta">${escapeHtml(config.command || "")}</span>
                          <span class="run-card-file">${escapeHtml(config.file_hint || "workspace")}</span>
                          <span class="run-card-task">${escapeHtml(config.detail || "")}</span>
                          ${
                            config.available === false
                              ? `<span class="run-card-task run-card-dependency">${escapeHtml(formatRunDependencyMessage(config))}</span>`
                              : ""
                          }
                        </button>
                        <details class="run-config-json">
                          <summary>launch / task</summary>
                          <pre>${escapeHtml(JSON.stringify({ launch: config.launch || {}, task: config.task || {} }, null, 2))}</pre>
                        </details>
                      </article>
                    `,
                  )
                  .join("")}
              </div>
            </section>
          `;
        })
        .join("")
    : `<div class="git-empty">${escapeHtml(currentLanguage === "zh" ? "未检测到可运行配置。" : "No runnable configuration detected.")}</div>`;

  runDebugSession.hidden = !active;
  runDebugSession.innerHTML = active
    ? `
      <div class="run-session-head">
        <strong>${escapeHtml(active.title || "")}</strong>
        <button class="git-inline-action is-danger" type="button" data-run-debug-stop="true">${escapeHtml(currentLanguage === "zh" ? "停止" : "Stop")}</button>
      </div>
      <div class="run-session-meta">PID ${escapeHtml(String(active.pid || ""))} / ${escapeHtml(active.started_at || "")}</div>
      <div class="run-session-meta">${escapeHtml(active.command || "")}</div>
      <button class="run-session-cwd" type="button" data-open-workspace-file=".">${escapeHtml(active.cwd || "")}</button>
      <div class="run-session-log">
        <div class="run-session-log-title">stdout</div>
        <pre>${escapeHtml(active.stdout_tail || "")}</pre>
      </div>
      <div class="run-session-log">
        <div class="run-session-log-title">stderr</div>
        <pre>${escapeHtml(active.stderr_tail || "")}</pre>
      </div>
    `
    : "";

  runDebugList.querySelectorAll("[data-run-config-id]").forEach((button) => {
    button.addEventListener("click", async () => {
      const configId = button.getAttribute("data-run-config-id") || "";
      if (!configId) return;
      runWorkspaceConfigAction(configId);
    });
  });
  bindTurnInteractionHandlers(runDebugSession);

  runDebugSession.querySelectorAll("[data-run-debug-stop]").forEach((button) => {
    button.addEventListener("click", async () => {
      try {
        await runRunDebugAction("stop");
      } catch (error) {
        console.error(error);
        showToast(appErrorMessage(error, "workspace", "toastSendFailed"));
      }
    });
  });
}

function setSettingsTab(tab) {
  activeSettingsTab = tab || "model";
  settingsTabs.forEach((button) => {
    button.classList.toggle("is-active", button.dataset.settingsTab === activeSettingsTab);
  });
  settingsTabPanels.forEach((panel) => {
    panel.classList.toggle("is-active", panel.dataset.settingsTabPanel === activeSettingsTab);
  });
}

function renderPendingFiles() {
  if (!composerAttachments) return;
  const composerShell = composerAttachments.closest(".composer-shell");
  if (!pendingFiles.length) {
    composerAttachments.hidden = true;
    composerAttachments.innerHTML = "";
    composerShell?.classList.remove("has-attachments");
    return;
  }

  composerAttachments.hidden = false;
  composerShell?.classList.add("has-attachments");
  composerAttachments.innerHTML = "";
  pendingFiles.forEach((file, index) => {
    const chip = document.createElement("div");
    chip.className = `attachment-chip${file.isImage ? " is-image" : " is-file"}`;
    const previewMarkup = file.isImage && file.previewUrl
      ? `<img class="attachment-preview" src="${escapeHtml(file.previewUrl)}" alt="" />`
      : `<span class="attachment-file-icon" aria-hidden="true"><svg viewBox="0 0 24 24"><path d="M7 3.5h6.8L18 7.7v12.8H7z"></path><path d="M13.5 3.5v4.4H18"></path></svg><span>${escapeHtml(file.extension || "FILE")}</span></span>`;
    chip.innerHTML = `
      ${previewMarkup}
      <span class="attachment-meta">
        <strong>${escapeHtml(file.name || `file-${index + 1}`)}</strong>
        <small>${escapeHtml(file.isImage ? "Image" : (file.extension || "FILE"))}</small>
      </span>
      <button class="attachment-remove" type="button" aria-label="Remove">x</button>
    `;
    chip.querySelector(".attachment-remove")?.addEventListener("click", () => {
      const removed = pendingFiles[index];
      if (removed?.previewUrl) URL.revokeObjectURL(removed.previewUrl);
      pendingFiles = pendingFiles.filter((_item, fileIndex) => fileIndex !== index);
      renderPendingFiles();
    });
    composerAttachments.appendChild(chip);
  });
}

function attachmentExtension(name) {
  const match = String(name || "").match(/\.([^.]+)$/);
  return match ? match[1].slice(0, 8).toUpperCase() : "FILE";
}

function isImageAttachment(file) {
  return String(file?.type || "").toLowerCase().startsWith("image/");
}

function readFileAsDataUrl(file) {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result || ""));
    reader.onerror = () => reject(reader.error || new Error("file read failed"));
    reader.readAsDataURL(file);
  });
}

function addPendingFiles(files) {
  Array.from(files || []).filter((file) => file instanceof File).forEach((file) => {
    const duplicate = pendingFiles.some((item) => item.name === file.name && item.size === file.size && item.lastModified === file.lastModified);
    if (duplicate) return;
    const isImage = isImageAttachment(file);
    pendingFiles.push({
      file,
      name: file.name,
      type: file.type || "application/octet-stream",
      size: file.size,
      lastModified: file.lastModified,
      extension: attachmentExtension(file.name),
      isImage,
      previewUrl: isImage ? URL.createObjectURL(file) : "",
    });
  });
  renderPendingFiles();
}

async function serializePendingAttachments() {
  return Promise.all(pendingFiles.map(async (item) => ({
    name: item.name,
    mime_type: item.type || "application/octet-stream",
    size: item.size || 0,
    data_url: await readFileAsDataUrl(item.file),
  })));
}

function sanitizeMessageContent(text) {
  const raw = String(text || "");
  const dsmlStarts = [
    "<||DSML",
    "</||DSML",
    "||DSML||",
    "<闁挎繃绮ｇ紞鎿燬ML",
    "</闁挎繃绮ｇ紞鎿燬ML",
    "闁挎繃绮ｇ紞鎿燬ML闁挎繃绮ｇ紞",
    "<DSML",
    "</DSML",
    "<tool_call",
    "</tool_call",
    "<function=",
    "<function ",
    "<function_",
    "</function>",
  ];
  let cutIndex = -1;
  for (const marker of dsmlStarts) {
    const index = raw.indexOf(marker);
    if (index !== -1 && (cutIndex === -1 || index < cutIndex)) {
      cutIndex = index;
    }
  }
  const toolNarrationMarkers = [
    "\nTool",
    "\r\nTool",
    "\nArguments",
    "\r\nArguments",
    "\nResult summary",
    "\r\nResult summary",
    "\n{\"operation\"",
    "\r\n{\"operation\"",
    "\n<tool_call",
    "\r\n<tool_call",
    "\n<function=",
    "\r\n<function=",
  ];
  for (const marker of toolNarrationMarkers) {
    const index = raw.indexOf(marker);
    if (index !== -1 && (cutIndex === -1 || index < cutIndex)) {
      cutIndex = index;
    }
  }
  for (const marker of ["Tool", "Arguments", "Result summary", "{\"operation\"", "<tool_call", "<function="]) {
    if (raw.startsWith(marker)) {
      cutIndex = cutIndex === -1 ? 0 : Math.min(cutIndex, 0);
    }
  }
  const visible = cutIndex >= 0 ? raw.slice(0, cutIndex) : raw;
  return visible
    .replace(/^\s*\[AGENT\]\s*/i, "")
    .replace(/^[\r\n]+/, "")
    .trimStart();
}

function createEmptyAssistantTurn() {
  return {
    text: "",
    streamingAnchorText: "",
    suppressedInlineContent: false,
    isThinkingPhase: false,
    runtimeNarration: "",
    progressNarration: "",
    assistantChoices: null,
    auto_skills: [],
    thinking: [],
    textSegments: [],
    streamMoments: [],
    process: [],
    worklog: [],
    processDelegates: [],
    subagents: [],
    verifierReport: null,
    runtimeCheckpoints: [],
    branchNotes: [],
    timeline: [],
    tools: [],
    diffs: [],
    permission: null,
    activity: "",
    startedAt: Date.now(),
    textUpdatedAt: 0,
    lastStreamEventKind: "",
    receivedDelta: false,
  };
}

function cloneAssistantTurnState(turn) {
  if (!turn) return null;
  return {
    ...turn,
    streamingAnchorText: String(turn.streamingAnchorText || ""),
    suppressedInlineContent: Boolean(turn.suppressedInlineContent),
    isThinkingPhase: Boolean(turn.isThinkingPhase),
    runtimeNarration: String(turn.runtimeNarration || ""),
    progressNarration: String(turn.progressNarration || ""),
    assistantChoices: turn.assistantChoices ? { ...turn.assistantChoices, options: Array.isArray(turn.assistantChoices.options) ? turn.assistantChoices.options.slice() : [] } : null,
    auto_skills: Array.isArray(turn.auto_skills) ? turn.auto_skills.map((item) => ({ ...item })) : [],
    thinking: Array.isArray(turn.thinking) ? turn.thinking.map((item) => ({ ...item })) : [],
    textSegments: Array.isArray(turn.textSegments) ? turn.textSegments.map((item) => ({ ...item })) : [],
    streamMoments: Array.isArray(turn.streamMoments) ? turn.streamMoments.map((item) => ({ ...item })) : [],
    process: Array.isArray(turn.process) ? turn.process.map((item) => ({ ...item })) : [],
    worklog: Array.isArray(turn.worklog) ? turn.worklog.map((item) => ({ ...item })) : [],
    processDelegates: Array.isArray(turn.processDelegates)
      ? turn.processDelegates.map((item) => ({ ...item }))
      : [],
    subagents: dedupeSubagentEntries(turn.subagents),
    verifierReport: turn.verifierReport
      ? {
          ...turn.verifierReport,
          checks: Array.isArray(turn.verifierReport.checks)
            ? turn.verifierReport.checks.map((item) => ({ ...item }))
            : [],
          issues: Array.isArray(turn.verifierReport.issues)
            ? turn.verifierReport.issues.slice()
            : [],
          evidence: Array.isArray(turn.verifierReport.evidence)
            ? turn.verifierReport.evidence.slice()
            : [],
          next_actions: Array.isArray(turn.verifierReport.next_actions)
            ? turn.verifierReport.next_actions.slice()
            : [],
        }
      : null,
    runtimeCheckpoints: Array.isArray(turn.runtimeCheckpoints)
      ? turn.runtimeCheckpoints.slice()
      : [],
    branchNotes: Array.isArray(turn.branchNotes) ? turn.branchNotes.slice() : [],
    timeline: Array.isArray(turn.timeline) ? turn.timeline.map((item) => ({ ...item })) : [],
    tools: Array.isArray(turn.tools) ? turn.tools.map((item) => ({ ...item })) : [],
    diffs: Array.isArray(turn.diffs) ? turn.diffs.map((item) => ({ ...item })) : [],
    permission: turn.permission ? { ...turn.permission } : null,
    textUpdatedAt: Number(turn.textUpdatedAt || 0) || 0,
    lastStreamEventKind: String(turn.lastStreamEventKind || ""),
  };
}

function mergeAssistantText(baseText, liveText, { preferLiveText = false } = {}) {
  const base = sanitizeMessageContent(String(baseText || "")).trim();
  const live = sanitizeMessageContent(String(liveText || "")).trim();
  if (!base) return live;
  if (!live) return base;
  if (base === live) return base;
  if (base.includes(live)) return base;
  if (live.includes(base)) return live;
  if (base.startsWith(live)) return base;
  if (live.startsWith(base)) return live;
  return preferLiveText
    ? combineAssistantSegments(live, base)
    : combineAssistantSegments(base, live);
}

function richerAssistantCollection(baseValue, liveValue) {
  const base = Array.isArray(baseValue) ? baseValue : [];
  const live = Array.isArray(liveValue) ? liveValue : [];
  if (!live.length) return base.map((item) => ({ ...item }));
  if (!base.length) return live.map((item) => ({ ...item }));
  return (live.length >= base.length ? live : base).map((item) => ({ ...item }));
}

function richerStringList(baseValue, liveValue) {
  const base = Array.isArray(baseValue) ? baseValue : [];
  const live = Array.isArray(liveValue) ? liveValue : [];
  if (!live.length) return base.slice();
  if (!base.length) return live.slice();
  return (live.length >= base.length ? live : base).slice();
}

function mergeAssistantTurnData(baseTurn, liveTurn, options = {}) {
  const base = cloneAssistantTurnState(baseTurn) || createEmptyAssistantTurn();
  const live = cloneAssistantTurnState(liveTurn);
  if (!live) return base;
  return {
    ...base,
    suppressedInlineContent: Boolean(base.suppressedInlineContent || live.suppressedInlineContent),
    runtimeNarration: mergeAssistantText(base.runtimeNarration, live.runtimeNarration, options),
    progressNarration: mergeAssistantText(base.progressNarration, live.progressNarration, options),
    assistantChoices: live.assistantChoices || base.assistantChoices || null,
    auto_skills: [],
    text: mergeAssistantText(base.text, live.text, options),
    thinking: richerAssistantCollection(base.thinking, live.thinking),
    textSegments: richerAssistantCollection(base.textSegments, live.textSegments),
    streamMoments: richerAssistantCollection(base.streamMoments, live.streamMoments),
    process: richerAssistantCollection(base.process, live.process),
    worklog: richerAssistantCollection(base.worklog, live.worklog),
    processDelegates: richerAssistantCollection(base.processDelegates, live.processDelegates),
    subagents: dedupeSubagentEntries([...(base.subagents || []), ...(live.subagents || [])]),
    verifierReport: live.verifierReport || base.verifierReport,
    runtimeCheckpoints: richerStringList(base.runtimeCheckpoints, live.runtimeCheckpoints),
    branchNotes: richerStringList(base.branchNotes, live.branchNotes),
    timeline: richerAssistantCollection(base.timeline, live.timeline),
    tools: richerAssistantCollection(base.tools, live.tools),
    diffs: richerAssistantCollection(base.diffs, live.diffs),
    permission: live.permission || base.permission,
    activity: String(live.activity || base.activity || ""),
    startedAt: live.startedAt || base.startedAt || Date.now(),
    textUpdatedAt: live.textUpdatedAt || base.textUpdatedAt || 0,
    lastStreamEventKind: String(live.lastStreamEventKind || base.lastStreamEventKind || ""),
    receivedDelta: Boolean(base.receivedDelta || live.receivedDelta),
  };
}

function shouldSuppressInlineAssistantCode(text, diffs = []) {
  const content = String(text || "");
  if (isAssistantCompletionSummaryText(content)) return false;
  const looksCodeLike = /```|(?:^|\n)\s*(?:def |class |function |async function |import |from |const |let |var |pub |fn |use |mod |impl |struct |enum |interface |type |#include |<\/?[a-z][^>]*>)/m.test(content);
  const codeFenceCount = (content.match(/```/g) || []).length;
  const codeyLineCount = content
    .split(/\r?\n/)
    .filter((line) =>
      /^\s*(?:def |class |function |async function |import |from |const |let |var |pub |fn |use |mod |impl |struct |enum |interface |type |#include |<\/?[a-z][^>]*>)/.test(line),
    )
    .length;
  const longCodeLike = looksCodeLike && (
    content.trim().length > 420
      || codeFenceCount >= 2
      || codeyLineCount >= 8
  );
  const looksJsonLike = (
    /^[\[{]/.test(content.trim())
    && /"(?:path|content|children|kind|role|type|tool|tool_args|call_id|status|result|data|delta|diff|before_content|after_content|session_id|runtime_snapshots|edited_files|tool_events)"/.test(content)
  ) || (
    content.trim().length > 240
    && /"(?:path|content|children|kind|role|type|tool|tool_args|call_id|status|result|data|delta|diff|before_content|after_content|session_id|runtime_snapshots|edited_files|tool_events)"/.test(content)
    && /[:[{[]/.test(content)
  );
  const looksPayloadLike = looksLikeOperationalContentDump(content)
    || looksLikeToolPayloadDump(content)
    || looksLikeDirectoryTreeDump(content);
  const hasDiffs = Array.isArray(diffs) && diffs.length > 0;
  return Boolean(
    looksPayloadLike
    || looksJsonLike
    || (longCodeLike && (hasDiffs || content.trim().length > 420))
  );
}

function turnHasRealWorkspaceChanges(turn) {
  if (!turn) return false;
  const diffs = Array.isArray(turn.diffs) ? turn.diffs : [];
  if (diffs.some((diff) => cleanDisplayText(diff?.path || "", ""))) return true;
  const tools = Array.isArray(turn.tools) ? turn.tools : [];
  return tools.some((tool) => {
    const name = String(tool?.name || "").toLowerCase();
    const status = String(tool?.status || "").toLowerCase();
    return ["write_file", "apply_patch", "search_and_replace", "search_and_replace_multi", "rename_path", "mkdir"].includes(name)
      && ["complete", "completed", "success", "succeeded"].includes(status)
      && Boolean(cleanDisplayText(tool?.file_path || tool?.params?.file_path || tool?.params?.path || tool?.params?.target_file || "", ""));
  });
}

function visibleAssistantWorkspaceNotice(turn = activeAssistantTurn) {
  if (!turnHasRealWorkspaceChanges(turn)) {
    return currentLanguage === "zh"
      ? "\u5df2\u7701\u7565\u4e0d\u9002\u5408\u4f5c\u4e3a\u804a\u5929\u6b63\u6587\u5c55\u793a\u7684\u4ee3\u7801\u6216\u5de5\u5177\u8f7d\u8377\u3002"
      : "Code or tool payload unsuitable for the chat body was omitted.";
  }
  return currentLanguage === "zh"
    ? "本轮结果已直接写入工作区文件，聊天区域不再展开完整源码。"
    : "This turn was written directly into workspace files. Full source is not expanded in chat.";
}

function zhLabel(zh, en) {
  return currentLanguage === "zh" ? zh : en;
}

function autoSkillKindLabel(kind) {
  const normalized = cleanDisplayText(kind || "", "general").toLowerCase();
  if (normalized === "workflow") return t("autoSkillsKindWorkflow");
  if (normalized === "subfield") return t("autoSkillsKindSubfield");
  return t("autoSkillsKindGeneral");
}

function registerUndoSnapshot(diff) {
  const key = `undo-${Date.now()}-${undoSnapshotSequence += 1}`;
  undoSnapshotCache.set(key, {
    path: String(diff?.path || ""),
    beforeContent: String(diff?.before_content || ""),
  });
  return key;
}

function diffStatusKey(diff) {
  return String(diff?.path || "");
}

function getDiffReviewStatus(diff) {
  return diffReviewState.get(diffStatusKey(diff)) || "pending";
}

function setDiffReviewStatus(diff, status) {
  diffReviewState.set(diffStatusKey(diff), status);
}

function renderDiffReviewStatus(status) {
  const normalized = String(status || "pending").toLowerCase();
  if (normalized === "accepted") {
    return currentLanguage === "zh" ? "已接受" : "Accepted";
  }
  if (normalized === "reverted") {
    return currentLanguage === "zh" ? "已撤销" : "Reverted";
  }
  return currentLanguage === "zh" ? "待审查" : "Pending review";
}

function renderAssistantDiffCard(diff, options = {}) {
  if (!diff?.path) return "";
  const inline = options.inline === true;
  const undoKey = registerUndoSnapshot(diff);
  const reviewOpen = activeReviewFilePath === diff.path;
  const reviewDetail = reviewOpen ? reviewDetailCache.get(diff.path) : null;
  const reviewStatus = getDiffReviewStatus(diff);
  const reviewMarkup = reviewOpen
    ? reviewDetail && !reviewDetail.error
      ? renderReviewDetail(reviewDetail, {
          path: diff.path,
          additions: Number(diff.added || 0),
          deletions: Number(diff.removed || 0),
        })
      : reviewDetail?.error
        ? `<div class="review-detail-error">${escapeHtml(reviewDetail.error)}</div>`
        : `<div class="review-detail-empty">${escapeHtml(t("reviewLoading"))}</div>`
    : "";
  const fileName = displayFileNameOnly(diff.path);
  const showPathMeta = fileName && fileName !== diff.path;

  if (inline) {
    return `
      <div class="codex-diff-card codex-diff-card-inline codex-diff-card-${escapeHtml(reviewStatus)}">
        <div class="codex-diff-card-head">
          <button
            class="codex-diff-chip codex-diff-chip-inline"
            type="button"
            data-open-workspace-file="${escapeHtml(diff.path)}"
            data-open-workspace-line="1"
            data-open-workspace-column="1"
            title="${escapeHtml(diff.path)}"
          >
            <span class="codex-diff-path">${escapeHtml(fileName || diff.path)}</span>
            <span class="codex-diff-stats">+${escapeHtml(String(diff.added || 0))} / -${escapeHtml(String(diff.removed || 0))}</span>
          </button>
          <span class="codex-diff-status codex-diff-status-${escapeHtml(reviewStatus)}">${escapeHtml(renderDiffReviewStatus(reviewStatus))}</span>
        </div>
        ${showPathMeta ? `<div class="codex-diff-path-meta">${escapeHtml(diff.path)}</div>` : ""}
        <div class="codex-diff-actions">
          <button class="codex-diff-open" type="button" data-review-path="${escapeHtml(diff.path)}">${escapeHtml(currentLanguage === "zh" ? "查看 diff" : "View diff")}</button>
          ${reviewStatus === "accepted"
            ? ""
            : `<button class="codex-diff-open" type="button" data-accept-path="${escapeHtml(diff.path)}" data-accept-updated-at="${escapeHtml(String(diff.updated_at || 0))}">${escapeHtml(currentLanguage === "zh" ? "接受" : "Accept")}</button>`}
          <button class="codex-diff-open codex-diff-undo" type="button" data-undo-key="${escapeHtml(undoKey)}">${escapeHtml(currentLanguage === "zh" ? "撤销" : "Undo")}</button>
        </div>
        <details class="codex-diff-review-panel"${reviewOpen ? " open" : ""}>
          <summary class="codex-diff-review-summary" data-review-path="${escapeHtml(diff.path)}">${escapeHtml(currentLanguage === "zh" ? "展开详细 diff" : "Expand detailed diff")}</summary>
          <div class="codex-diff-review-body">${reviewMarkup || `<div class="review-detail-empty">${escapeHtml(t("reviewLoading"))}</div>`}</div>
        </details>
      </div>
    `;
  }

  return `
    <div class="codex-diff-step">
      <div class="codex-step-rail" aria-hidden="true">
        <span class="codex-step-dot"></span>
      </div>
      <div class="codex-diff-card codex-diff-card-${escapeHtml(reviewStatus)}">
        <div class="codex-diff-card-head">
          <button
            class="codex-diff-chip"
            type="button"
            data-open-workspace-file="${escapeHtml(diff.path)}"
            data-open-workspace-line="1"
            data-open-workspace-column="1"
          >
            <span class="codex-diff-path">${escapeHtml(diff.path)}</span>
            <span class="codex-diff-stats">+${escapeHtml(String(diff.added || 0))} / -${escapeHtml(String(diff.removed || 0))}</span>
          </button>
          <span class="codex-diff-status codex-diff-status-${escapeHtml(reviewStatus)}">${escapeHtml(renderDiffReviewStatus(reviewStatus))}</span>
          <div class="codex-diff-actions">
            <button class="codex-diff-open" type="button" data-review-path="${escapeHtml(diff.path)}">${escapeHtml(currentLanguage === "zh" ? "Review" : "Review")}</button>
            ${reviewStatus === "accepted"
              ? ""
              : `<button class="codex-diff-open" type="button" data-accept-path="${escapeHtml(diff.path)}" data-accept-updated-at="${escapeHtml(String(diff.updated_at || 0))}">${escapeHtml(currentLanguage === "zh" ? "接受" : "Accept")}</button>`}
            <button class="codex-diff-open codex-diff-undo" type="button" data-undo-key="${escapeHtml(undoKey)}">${escapeHtml(currentLanguage === "zh" ? "撤销" : "Undo")}</button>
          </div>
        </div>
        <div class="codex-diff-meta">${escapeHtml(currentLanguage === "zh" ? "Agent 刚更新了这个文件" : "Agent just updated this file")}</div>
        <details class="codex-diff-review-panel"${reviewOpen ? " open" : ""}>
          <summary class="codex-diff-review-summary" data-review-path="${escapeHtml(diff.path)}">${escapeHtml(currentLanguage === "zh" ? "查看 diff" : "View diff")}</summary>
          <div class="codex-diff-review-body">${reviewMarkup || `<div class="review-detail-empty">${escapeHtml(t("reviewLoading"))}</div>`}</div>
        </details>
      </div>
    </div>
  `;
}

function renderAssistantDiffMarkup(diffs, options = {}) {
  const visibleDiffs = Array.isArray(diffs) ? diffs.filter((diff) => diff?.path) : [];
  if (!visibleDiffs.length) return "";
  const inline = options.inline === true;
  const totalAdded = visibleDiffs.reduce((sum, diff) => sum + Number(diff.added || 0), 0);
  const totalRemoved = visibleDiffs.reduce((sum, diff) => sum + Number(diff.removed || 0), 0);
  return `
    <div class="codex-diff-list codex-steps-list${inline ? " codex-diff-list-inline" : ""}">
      <div class="codex-diff-summary${inline ? " is-inline" : ""}">
        <span class="codex-diff-summary-title">${escapeHtml(
          currentLanguage === "zh"
            ? (inline ? `刚刚更新了 ${visibleDiffs.length} 个文件` : `已修改 ${visibleDiffs.length} 个文件`)
            : (inline ? `${visibleDiffs.length} files just changed` : `${visibleDiffs.length} files changed`),
        )}</span>
        <span class="codex-diff-summary-stats">+${escapeHtml(String(totalAdded))} / -${escapeHtml(String(totalRemoved))}</span>
      </div>
      ${visibleDiffs.map((diff) => renderAssistantDiffCard(diff, { inline })).join("")}
    </div>
  `;
}

function syncAcceptedDiffStatuses(turn) {
  if (!turn?.verifierReport) return;
  const verifierStatus = String(turn.verifierReport.status || "").toLowerCase();
  if (!["pass", "complete"].includes(verifierStatus)) return;
  const diffs = Array.isArray(turn.diffs) ? turn.diffs : [];
  diffs.forEach((diff) => {
    if (diff?.path) {
      setDiffReviewStatus(diff, "accepted");
    }
  });
}

function syncAcceptedDiffStatusesFromMessages(messages) {
  const turns = groupMessagesIntoTurns(visibleConversationMessages(messages || []));
  turns.forEach((turn) => {
    if (turn?.kind === "assistant_turn" && turn?.data) {
      syncAcceptedDiffStatuses(turn.data);
    }
  });
}

function resetActiveAssistantTurn() {
  activeAssistantTurn = createEmptyAssistantTurn();
}

function getSessionRunState(sessionId) {
  const key = String(sessionId || "").trim();
  if (!key) return null;
  if (!sessionRunState.has(key)) {
    sessionRunState.set(key, {
      sessionId: key,
      generation: 0,
      running: false,
      waitingApproval: false,
      updatedAt: 0,
    });
  }
  return sessionRunState.get(key);
}

function beginSessionRun(sessionId) {
  const state = getSessionRunState(sessionId);
  if (!state) return null;
  state.generation += 1;
  state.running = true;
  state.waitingApproval = false;
  state.updatedAt = Date.now();
  renderCurrentSession(bootstrapData?.sessions || [], bootstrapData?.current_session_id || null);
  renderSessionList(bootstrapData?.sessions || [], bootstrapData?.current_session_id || null);
  return state.generation;
}

function touchSessionRun(sessionId, updates = {}) {
  const state = getSessionRunState(sessionId);
  if (!state) return;
  Object.assign(state, updates, { updatedAt: Date.now() });
  renderCurrentSession(bootstrapData?.sessions || [], bootstrapData?.current_session_id || null);
  renderSessionList(bootstrapData?.sessions || [], bootstrapData?.current_session_id || null);
}

function endSessionRun(sessionId) {
  const state = getSessionRunState(sessionId);
  if (!state) return;
  state.running = false;
  state.waitingApproval = false;
  state.updatedAt = Date.now();
  clearVisibleRuntimeSnapshot(sessionId);
  renderCurrentSession(bootstrapData?.sessions || [], bootstrapData?.current_session_id || null);
  renderSessionList(bootstrapData?.sessions || [], bootstrapData?.current_session_id || null);
}

function bootstrapHasLiveSession(data, sessionId) {
  const targetSessionId = String(sessionId || "").trim();
  if (!targetSessionId) return false;
  const activeMatch = (Array.isArray(data?.active_sessions) ? data.active_sessions : [])
    .some((item) => String(item?.session_id || "").trim() === targetSessionId);
  if (activeMatch) return true;
  return (Array.isArray(data?.runtime_snapshots) ? data.runtime_snapshots : [])
    .some((item) => String(item?.session_id || "").trim() === targetSessionId);
}

function clearVisibleRuntimeSnapshot(sessionId = null) {
  const targetSessionId = String(sessionId || bootstrapData?.current_session_id || "").trim();
  if (!targetSessionId || !bootstrapData) return;
  bootstrapData = {
    ...bootstrapData,
    active_sessions: (Array.isArray(bootstrapData.active_sessions) ? bootstrapData.active_sessions : [])
      .filter((item) => String(item?.session_id || "").trim() !== targetSessionId),
    runtime_snapshots: (Array.isArray(bootstrapData.runtime_snapshots) ? bootstrapData.runtime_snapshots : [])
      .filter((item) => String(item?.session_id || "").trim() !== targetSessionId),
  };
}

function syncActiveSessionsFromBootstrap(data, options = {}) {
  const activeIds = new Set(
    (Array.isArray(data?.active_sessions) ? data.active_sessions : [])
      .map((item) => String(item?.session_id || "").trim())
      .filter(Boolean),
  );
  const preservedRunningSessionId = activeIds.has(String(options.preserveRunningSessionId || "").trim())
    ? String(options.preserveRunningSessionId || "").trim()
    : "";
  sessionRunState.forEach((state, sessionId) => {
    if (!activeIds.has(sessionId)) {
      if (preservedRunningSessionId && preservedRunningSessionId === sessionId && state.running) {
        return;
      }
      state.running = false;
      state.waitingApproval = false;
    }
  });
  (Array.isArray(data?.active_sessions) ? data.active_sessions : []).forEach((item) => {
    const sessionId = String(item?.session_id || "").trim();
    if (!sessionId) return;
    touchSessionRun(sessionId, {
      running: String(item?.status || "") === "running",
      waitingApproval: Boolean(item?.waiting_approval),
    });
  });
}

function hydrateVisibleRuntimeSnapshot() {
  const currentSessionId = String(bootstrapData?.current_session_id || "").trim();
  if (!currentSessionId) return;
  const runState = getSessionRunState(currentSessionId);
  if (!runState?.running) return;
  const snapshot = (Array.isArray(bootstrapData?.runtime_snapshots) ? bootstrapData.runtime_snapshots : [])
    .find((item) => String(item?.session_id || "").trim() === currentSessionId);
  if (!snapshot) return;

  currentStreamingSessionId = currentSessionId;
  if (!activeAssistantTurn) {
    resetActiveAssistantTurn();
  }
  const snapshotText = String(snapshot.partial_text || "");
  const currentText = String(activeAssistantTurn.text || "");
  const preserveStreamingText = shouldPreserveStreamingConversationDom();
  if (!preserveStreamingText) {
    activeAssistantTurn.text = snapshotText;
  } else if (!currentText && !activeAssistantTurn.receivedDelta) {
    activeAssistantTurn.text = snapshotText;
  }
  activeAssistantTurn.process = snapshot.latest_activity
    ? [{
        id: `${currentSessionId}-runtime`,
        type: snapshot.latest_activity.label || "activity",
        label: snapshot.latest_activity.label || "",
        detail: snapshot.latest_activity.detail || "",
        meta: [snapshot.latest_activity.agent, snapshot.latest_activity.phase, snapshot.latest_activity.status]
          .filter(Boolean)
          .join(" / "),
        phase: snapshot.latest_activity.phase || "",
        status: snapshot.latest_activity.status || "",
        agent: snapshot.latest_activity.agent || "",
      }]
    : [];
  activeAssistantTurn.processDelegates = Array.isArray(snapshot.latest_activity?.delegates)
    ? snapshot.latest_activity.delegates.map((delegate) => ({ ...delegate }))
    : [];
  activeAssistantTurn.worklog = [];
  activeAssistantTurn.runtimeNarration = "";
  const snapshotActivityWorklog = snapshot.latest_activity
    ? describeActivityWorklog({
        label: snapshot.latest_activity.label || "",
        detail: snapshot.latest_activity.detail || "",
        meta: [snapshot.latest_activity.agent, snapshot.latest_activity.phase, snapshot.latest_activity.status]
          .filter(Boolean)
          .join(" / "),
        phase: snapshot.latest_activity.phase || "",
        status: snapshot.latest_activity.status || "",
        agent: snapshot.latest_activity.agent || "",
      })
    : null;
  if (snapshotActivityWorklog) {
    pushAssistantWorklog(snapshotActivityWorklog);
    updateRuntimeNarration(snapshot.latest_activity?.detail || snapshot.latest_activity?.meta || "");
  }
  (Array.isArray(snapshot.progress_updates) ? snapshot.progress_updates : []).forEach((entry) => {
    pushAssistantProgressWorklogText(entry);
    updateRuntimeNarration(entry);
  });
  activeAssistantTurn.subagents = Array.isArray(snapshot.subagents)
    ? dedupeSubagentEntries(snapshot.subagents)
    : [];
  activeAssistantTurn.auto_skills = [];
  activeAssistantTurn.verifierReport = snapshot.verifier ? { ...snapshot.verifier } : null;
  activeAssistantTurn.runtimeCheckpoints = Array.isArray(snapshot.checkpoints)
    ? snapshot.checkpoints.slice()
    : [];
  activeAssistantTurn.branchNotes = normalizeBranchNotes(snapshot.branch_notes);
  activeAssistantTurn.timeline = Array.isArray(snapshot.timeline)
    ? snapshot.timeline.map((item) => ({ ...item }))
    : [];
  activeAssistantTurn.tools = Array.isArray(snapshot.tool_events)
    ? snapshot.tool_events.map((tool) => ({ ...tool }))
    : [];
  activeAssistantTurn.diffs = Array.isArray(snapshot.edited_files)
    ? snapshot.edited_files.map((file) => ({
        path: file.path,
        added: Number(file.added || 0),
        removed: Number(file.removed || 0),
        before_content: String(file.before_content || ""),
        after_content: String(file.after_content || ""),
        updated_at: Date.now(),
      }))
    : [];
  if (!activeAssistantTurn.runtimeNarration) {
    updateRuntimeNarration(snapshot.latest_activity?.detail || snapshot.latest_activity?.label || "");
  }
  activeAssistantTurn.permission = snapshot.permission || null;
  pendingPermissionRequest = snapshot.permission || null;
  liveProcessEvents = activeAssistantTurn.process.map((event) => ({
    ...event,
    timestamp: Date.now(),
  }));
  liveToolEvents = activeAssistantTurn.tools.slice(-6);
  liveEditedFiles = activeAssistantTurn.diffs.slice(-6).map((diff) => ({
    path: diff.path,
    added: diff.added,
    removed: diff.removed,
  }));
  const hasStreamingRenderableContent = Boolean(
    String(activeAssistantTurn.text || "").trim() ||
    String(activeAssistantTurn.runtimeNarration || "").trim() ||
    activeAssistantTurn.worklog.length
  );
  if (
    getSessionRunState(currentSessionId)?.running &&
    (hasStreamingRenderableContent ||
      activeAssistantTurn.process.length ||
      activeAssistantTurn.processDelegates.length ||
      activeAssistantTurn.subagents.length ||
      activeAssistantTurn.verifierReport ||
      activeAssistantTurn.tools.length ||
      activeAssistantTurn.diffs.length ||
      activeAssistantTurn.permission)
  ) {
    if (!pendingAssistantBubble) {
      appendAssistantBubble(activeAssistantTurn.text || "");
    } else {
      refreshPendingAssistantBubble();
    }
    setStopButtonVisible(true);
  }
  renderAgentRuntimeStrip();
  renderAgentProcessStrip();
  renderPermissionStrip();
}

function syncActiveTurnRuntime(runtime) {
  if (!activeAssistantTurn || !runtime) return;
  activeAssistantTurn.runtimeCheckpoints = Array.isArray(runtime.checkpoints)
    ? runtime.checkpoints.slice()
    : activeAssistantTurn.runtimeCheckpoints;
  activeAssistantTurn.branchNotes = Array.isArray(runtime.branch_notes)
    ? normalizeBranchNotes(runtime.branch_notes)
    : activeAssistantTurn.branchNotes;
  activeAssistantTurn.timeline = Array.isArray(runtime.timeline)
    ? runtime.timeline.map((item) => ({ ...item }))
    : activeAssistantTurn.timeline;
  activeAssistantTurn.auto_skills = [];
}

function captureMessageScrollPosition() {
  if (!messageStream) return;
  const scrollTop = messageStream.scrollTop;
  const distanceFromBottom = Math.max(
    0,
    messageStream.scrollHeight - messageStream.clientHeight - scrollTop,
  );
  preservedMessageScrollState = {
    scrollTop,
    distanceFromBottom,
    stickToBottom: distanceFromBottom <= 72,
  };
}

function restoreMessageScrollPosition() {
  if (!messageStream || !preservedMessageScrollState) return;
  const nextScrollTop = preservedMessageScrollState.stickToBottom
    ? Math.max(0, messageStream.scrollHeight - messageStream.clientHeight - preservedMessageScrollState.distanceFromBottom)
    : Math.min(
      preservedMessageScrollState.scrollTop,
      Math.max(0, messageStream.scrollHeight - messageStream.clientHeight),
    );
  messageStream.scrollTop = nextScrollTop;
  preservedMessageScrollState = null;
}

function finalizeActiveAssistantTurn() {
  if (!activeAssistantTurn) return null;
  const snapshot = cloneAssistantTurnState(activeAssistantTurn);
  activeAssistantTurn.activity = "";
  activeAssistantTurn.permission = null;
  activeAssistantTurn.process = [];
  activeAssistantTurn.processDelegates = [];
  activeAssistantTurn.subagents = [];
  activeAssistantTurn.verifierReport = null;
  activeAssistantTurn.runtimeCheckpoints = [];
  activeAssistantTurn.branchNotes = [];
  activeAssistantTurn.timeline = [];
  activeAssistantTurn.tools = [];
  if (Array.isArray(activeAssistantTurn.diffs) && activeAssistantTurn.diffs.length) {
    pinnedEditedFiles = activeAssistantTurn.diffs
      .map((diff) => ({
        path: diff.path,
        added: Number(diff.added || 0),
        removed: Number(diff.removed || 0),
        updated_at: Number(diff.updated_at || Date.now()),
      }))
      .slice(-6);
  } else {
    pinnedEditedFiles = [];
  }
  resetActiveAssistantTurn();
  setStopButtonVisible(false);
  return snapshot;
}

function visibleConversationMessages(messages) {
  return (Array.isArray(messages) ? messages : []).filter(
    (message) =>
      message &&
      (message.kind === "message" ||
        message.kind === "thinking" ||
        message.kind === "tool_result" ||
        message.kind === "diff" ||
        message.kind === "subagent" ||
        message.kind === "verification"),
  );
}

function persistConversationMessages(messages, { sessionId = null } = {}) {
  const visibleMessages = visibleConversationMessages(messages);
  lastVisibleCompletionSignature = visibleMessagesSignature(visibleMessages);
  clearPendingAssistantFrames();
  bootstrapData = {
    ...(bootstrapData || {}),
    messages: visibleMessages,
    current_session_id: sessionId || bootstrapData?.current_session_id || null,
  };
  renderReview(buildReviewFromMessages(visibleMessages));
  syncAgentPreludeBackground(visibleMessages);
  renderMessages(visibleMessages);
}

function commitStreamFailure(error) {
  const nextMessages = [...visibleConversationMessages(bootstrapData?.messages || [])];
  const lastPersistedAssistant = [...nextMessages]
    .reverse()
    .find((message) => message?.kind === "message" && message?.role === "assistant");
  if (
    /(?:\u672c\u8f6e|\u8fd9\u8f6e).*(?:\u4e2d\u65ad|\u5931\u8d25|\u505c\u6b62|\u6682\u505c)|(?:interrupted|stopped early|internal execution component failed)/i
      .test(String(lastPersistedAssistant?.content || ""))
  ) {
    persistConversationMessages(nextMessages, { sessionId: currentStreamingSessionId });
    return;
  }
  const partialText = String(activeAssistantTurn?.text || "").trim();
  if (partialText) {
    const lastMessage = nextMessages[nextMessages.length - 1] || null;
    if (!(lastMessage && lastMessage.kind === "message" && lastMessage.role === "assistant" && String(lastMessage.content || "").trim() === partialText)) {
      nextMessages.push({
        kind: "message",
        role: "assistant",
        content: partialText,
      });
    }
  }

  const classified = classifyAppError(error, "send");
  const failureText = currentLanguage === "zh"
    ? `\u672c\u8f6e\u63d0\u524d\u4e2d\u65ad\uff1a${classified.message || "\u53d1\u9001\u5931\u8d25\u3002"}`
    : `This turn stopped early: ${classified.message || "Send failed."}`;
  nextMessages.push({
    kind: "message",
    role: "assistant",
    content: failureText,
  });
  persistConversationMessages(nextMessages, { sessionId: currentStreamingSessionId });
}

function materializePendingConversationMessages({ sessionId = null } = {}) {
  const visibleMessages = [...visibleConversationMessages(bootstrapData?.messages || [])];
  const content = completionFallbackAssistantContent(activeAssistantTurn);
  const fallbackChoices = activeAssistantTurn?.assistantChoices
    && Array.isArray(activeAssistantTurn.assistantChoices.options)
    && activeAssistantTurn.assistantChoices.options.length
      ? {
          title: cleanDisplayText(activeAssistantTurn.assistantChoices.title || "", zhLabel("选择下一步", "Choose next step")),
          options: activeAssistantTurn.assistantChoices.options
            .map((item) => cleanDisplayText(item || "", ""))
            .filter(Boolean),
        }
      : null;
  if (content || fallbackChoices) {
    const contentCore = cleanDisplayText(assistantPrimaryReplyCore(content), "");
    const alreadyPresent = visibleMessages.some((message) =>
      messageHasAssistantChoices(message)
      || (message
        && message.kind === "message"
        && message.role === "assistant"
        && (() => {
          const messageText = cleanDisplayText(String(message.content || "").trim(), "");
          const messageCore = cleanDisplayText(assistantPrimaryReplyCore(messageText), "");
          return messageText === content
            || messageText.includes(content)
            || content.includes(messageText)
            || (contentCore && messageCore && (contentCore === messageCore || contentCore.includes(messageCore) || messageCore.includes(contentCore)));
        })()),
    );
    if (!alreadyPresent) {
      const message = {
        kind: "message",
        role: "assistant",
        content: content || "",
      };
      if (fallbackChoices) {
        message.assistant_choices = fallbackChoices;
      }
      visibleMessages.push(message);
    }
  }
  bootstrapData = {
    ...(bootstrapData || {}),
    messages: visibleMessages,
    current_session_id: sessionId || bootstrapData?.current_session_id || null,
  };
  renderReview(buildReviewFromMessages(visibleMessages));
  syncAgentPreludeBackground(visibleMessages);
  renderMessages(visibleMessages, { preserveScroll: true });
}

function resetConversationRuntimeState({ preserveInputFocus = false } = {}) {
  activeStreamGeneration += 1;
  pendingResearchStart = false;
  isSending = false;
  suppressVisibleStreamBootstrap = false;
  if (currentStreamingSessionId) {
    const state = getSessionRunState(currentStreamingSessionId);
    if (state) {
      state.generation = activeStreamGeneration;
      state.running = false;
      state.waitingApproval = false;
      state.updatedAt = Date.now();
    }
  }
  currentStreamingSessionId = null;
  clearVisibleRuntimeSnapshot();
  pendingPermissionRequest = null;
  liveToolEvents = [];
  liveEditedFiles = [];
  liveProcessEvents = [];
  pinnedEditedFiles = [];
  lastVisibleCompletionSignature = "";
  resetPendingAssistantRenderState();
  clearPendingAssistantFrames();
  if (pendingUserBubble) {
    pendingUserBubble.remove();
    pendingUserBubble = null;
  }
  if (pendingAssistantBubble) {
    pendingAssistantBubble.remove();
    pendingAssistantBubble = null;
  }
  resetActiveAssistantTurn();
  renderAgentRuntimeStrip();
  renderAgentProcessStrip();
  renderPermissionStrip();
  stopActivity();
  setStopButtonVisible(false);
  if (messageInput) {
    messageInput.disabled = false;
    if (preserveInputFocus) {
      messageInput.focus();
    }
  }
}

function pendingAssistantPlaceholderText() {
  return "";
}

function bindPendingAssistantNodes(scope = pendingAssistantBubble) {
  pendingAssistantRuntimeNode = scope?.querySelector("[data-pending-runtime-panel]") || scope?.querySelector("[data-streaming-runtime]") || null;
  pendingAssistantRuntimeStatusesNode = scope?.querySelector("[data-runtime-statuses]") || null;
  pendingAssistantStoryNode = scope?.querySelector("[data-turn-storyline]") || null;
  pendingAssistantOperationsNode = scope?.querySelector("[data-turn-operations]") || null;
  pendingAssistantThinkingHost = scope?.querySelector("[data-thinking-host]") || null;
  const markdownNodes = scope ? [...scope.querySelectorAll("[data-streaming-markdown]")] : [];
  const stableNodes = scope ? [...scope.querySelectorAll("[data-streaming-markdown-stable]")] : [];
  const tailNodes = scope ? [...scope.querySelectorAll("[data-streaming-markdown-tail]")] : [];
  pendingAssistantTextNode = markdownNodes.length ? markdownNodes[markdownNodes.length - 1] : null;
  pendingAssistantStableNode = stableNodes.length ? stableNodes[stableNodes.length - 1] : null;
  pendingAssistantTailNode = tailNodes.length ? tailNodes[tailNodes.length - 1] : null;
  pendingAssistantThinkingNode = scope?.querySelector(".codex-thinking-block .thinking-content") || null;
  pendingAssistantStatusTextNode = scope?.querySelector("[data-turn-activity]") || null;
  pendingAssistantStatusTimeNode = scope?.querySelector("[data-turn-elapsed]") || null;
  pendingAssistantRenderedRuntimeText = null;
  pendingAssistantRenderedStableText = null;
  pendingAssistantRenderedTailText = null;
  pendingAssistantRenderedOperationsHtml = null;
}

function bindPendingAssistantStoryNodes(scope = pendingAssistantStoryNode) {
  const markdownNodes = scope ? [...scope.querySelectorAll("[data-streaming-markdown]")] : [];
  const stableNodes = scope ? [...scope.querySelectorAll("[data-streaming-markdown-stable]")] : [];
  const tailNodes = scope ? [...scope.querySelectorAll("[data-streaming-markdown-tail]")] : [];
  pendingAssistantTextNode = markdownNodes.length ? markdownNodes[markdownNodes.length - 1] : null;
  pendingAssistantStableNode = stableNodes.length ? stableNodes[stableNodes.length - 1] : null;
  pendingAssistantTailNode = tailNodes.length ? tailNodes[tailNodes.length - 1] : null;
  pendingAssistantRenderedStableText = null;
  pendingAssistantRenderedTailText = null;
}

function syncPendingAssistantRuntimePanel() {
  if (!pendingAssistantRuntimeNode || !activeAssistantTurn) return;
  if (!autoOpenActivityPanel) {
    if (pendingAssistantRuntimeNode.firstChild) {
      pendingAssistantRuntimeNode.replaceChildren();
    }
    return;
  }
  const runtimeSummaryParts = summarizeTurnRuntime(activeAssistantTurn);
  const operationDetailPanels = buildOperationDetailPanels(activeAssistantTurn, { isStreaming: true });
  const permissionMarkup = activeAssistantTurn.permission
    ? `
      <div class="codex-approval-card codex-tool-step codex-approval-step">
        <div class="codex-step-rail" aria-hidden="true">
          <span class="codex-step-dot"></span>
        </div>
        <div class="codex-approval-copy">
          <div class="codex-approval-title">${escapeHtml(currentLanguage === "zh" ? "等待工具授权" : "Awaiting tool approval")}</div>
          <div class="codex-approval-meta">${escapeHtml(activeAssistantTurn.permission.name || "")}${activeAssistantTurn.permission.risk ? ` / ${escapeHtml(activeAssistantTurn.permission.risk || "")}` : ""}</div>
        </div>
        <div class="codex-approval-actions">
          <button class="codex-approval-button" type="button" data-permission-action="deny">${escapeHtml(currentLanguage === "zh" ? "拒绝" : "Deny")}</button>
          <button class="codex-approval-button is-primary" type="button" data-permission-action="approve">${escapeHtml(currentLanguage === "zh" ? "批准" : "Approve")}</button>
        </div>
      </div>
    `
    : "";
  const runtimePanelContent = [
    permissionMarkup,
    operationDetailPanels ? `<div class="codex-operation-detail-stack">${operationDetailPanels}</div>` : "",
  ].filter(Boolean).join("");
  const shouldShowRuntimePanel = Boolean(runtimePanelContent);
  const nextHtml = shouldShowRuntimePanel
    ? renderAssistantRuntimePanel(runtimePanelContent, {
        title: currentLanguage === "zh" ? "操作" : "Activity",
        meta: runtimeSummaryParts.join(" / "),
        open: true,
        tone: "running",
      })
    : "";
  if (pendingAssistantRuntimeNode.innerHTML !== nextHtml) {
    pendingAssistantRuntimeNode.innerHTML = nextHtml;
    bindTurnInteractionHandlers(pendingAssistantBubble);
  }
}

function ensurePendingThinkingExpanded() {
  if (!pendingAssistantBubble) return;
  pendingAssistantBubble.querySelectorAll(".codex-thinking-block").forEach((details) => {
    const shell = details;
    if (shell instanceof HTMLDetailsElement) {
      shell.open = true;
    }
  });
}

function renderStreamingThinkingIndicator() {
  return `
    <section class="thinking-block codex-thinking-block is-thinking" data-thinking-streaming>
      <div class="codex-thinking-summary" role="status" aria-live="polite">${renderThinkingSummaryLabel(0, true)}</div>
    </section>
  `;
}

function hasPendingAssistantShell(scope = pendingAssistantBubble) {
  return Boolean(scope?.querySelector?.("[data-pending-assistant-shell]"));
}

function renderPendingAssistantShell() {
  return `<div class="codex-stream-phase"${streamAnimationStyle()}>
    <article class="message-row assistant-row assistant-message-row codex-turn-row" data-pending-assistant-shell>
      <div class="codex-turn-shell">
        <div data-thinking-host></div>
        <div class="codex-answer codex-answer-streaming codex-answer-empty">
          <div data-turn-storyline></div>
          <div class="codex-runtime-statuses" data-runtime-statuses hidden></div>
        </div>
        <div class="codex-turn-operations" data-turn-operations hidden></div>
        <div data-pending-runtime-panel></div>
      </div>
    </article>
  </div>`;
}

function ensurePendingAssistantShellBound() {
  if (!pendingAssistantBubble) return false;
  if (!hasPendingAssistantShell()) {
    pendingAssistantBubble.innerHTML = renderPendingAssistantShell();
    bindTurnInteractionHandlers(pendingAssistantBubble);
  }
  if (!pendingAssistantThinkingHost || !pendingAssistantStoryNode || !pendingAssistantOperationsNode || !pendingAssistantRuntimeStatusesNode) {
    bindPendingAssistantNodes(pendingAssistantBubble);
  }
  return true;
}

function syncPendingThinkingIndicator() {
  if (!pendingAssistantThinkingHost || !activeAssistantTurn) return;
  const shouldShow = Boolean(activeAssistantTurn.isThinkingPhase);
  const existing = pendingAssistantThinkingHost.querySelector("[data-thinking-streaming]");
  if (shouldShow && !existing) {
    pendingAssistantThinkingHost.insertAdjacentHTML("beforeend", renderStreamingThinkingIndicator());
  } else if (!shouldShow && existing) {
    existing.remove();
  }
}

function syncPendingAssistantOperations() {
  if (!pendingAssistantOperationsNode) return;
  if (pendingAssistantOperationsNode.firstChild) pendingAssistantOperationsNode.replaceChildren();
  pendingAssistantRenderedOperationsHtml = "";
  pendingAssistantOperationsNode.hidden = true;
}

function isStreamingBlockLine(line) {
  const trimmed = String(line || "").trimStart();
  if (!trimmed) return false;
  return /^(#{1,6})(\s|$)/.test(trimmed)
    || /^[-*+]\s+/.test(trimmed)
    || /^\d+\.\s+/.test(trimmed)
    || /^>\s?/.test(trimmed)
    || /^\|.+\|?$/.test(trimmed)
    || /^(?:---+|___+|\*\*\*+)\s*$/.test(trimmed);
}

function findStreamingRenderableBoundary(text) {
  const source = String(text || "").replace(/\r\n/g, "\n");
  if (!source) return 0;

  const lines = source.split("\n");
  let cursor = 0;
  let inFence = false;
  let openFenceIndex = -1;

  for (const line of lines) {
    if (/^\s*```/.test(line)) {
      if (!inFence) {
        inFence = true;
        openFenceIndex = cursor;
      } else {
        inFence = false;
        openFenceIndex = -1;
      }
    }
    cursor += line.length + 1;
  }

  if (inFence && openFenceIndex >= 0) {
    return openFenceIndex;
  }

  if (source.endsWith("\n")) {
    return source.length;
  }

  const lastLineBreak = source.lastIndexOf("\n");
  const trailingLine = lastLineBreak >= 0 ? source.slice(lastLineBreak + 1) : source;
  if (isStreamingBlockLine(trailingLine)) {
    return Math.max(0, lastLineBreak + 1);
  }
  if (lastLineBreak >= 0) {
    return lastLineBreak + 1;
  }

  return source.length;
}

function prepareStreamingMarkdownContent(input) {
  const source = sanitizeMessageContent(String(input || ""))
    .replace(/\r\n/g, "\n")
    .replace(/\s+---+\s+/g, "\n\n")
    .trimStart();
  if (!source) return "";

  const protectedBlocks = protectFencedMarkdownBlocks(repairCollapsedStreamingStructure(source));
  const normalized = protectedBlocks.text
    .split("\n")
    .flatMap((line) => splitResearchStepNarrationLine(line))
    .map((line) => normalizeAssistantStructuralLine(line))
    .join("\n");
  return restoreFencedMarkdownBlocks(normalized, protectedBlocks.blocks);
}

function renderStreamingAssistantContent(content, options = {}) {
  const streaming = Boolean(options.streaming);
  const text = streaming
    ? prepareStreamingMarkdownContent(String(content || ""))
    : structureAssistantDisplayText(String(content || ""));
  const placeholder = options.placeholder || pendingAssistantPlaceholderText();
  if (!text) {
    return {
      html: escapeHtml(placeholder),
      stableHtml: "",
      tailText: "",
      isEmpty: true,
    };
  }
  if (streaming) {
    const boundary = Math.max(0, Math.min(findStreamingRenderableBoundary(text), text.length));
    const stableSource = text.slice(0, boundary).trimEnd();
    const tailText = text.slice(boundary);
    const stableHtml = stableSource ? renderMarkdown(stableSource) : "";
    return {
      html: `${stableHtml}${tailText ? `<pre class="codex-streaming-tail">${escapeHtml(tailText)}</pre>` : ""}`,
      stableHtml,
      tailText,
      isEmpty: !stableHtml && !tailText,
    };
  }
  return {
    html: renderMarkdown(text),
    stableHtml: renderMarkdown(text),
    tailText: "",
    isEmpty: false,
  };
}

function resetPendingAssistantRenderState() {
  pendingAssistantRuntimeNode = null;
  pendingAssistantRuntimeStatusesNode = null;
  pendingAssistantStoryNode = null;
  pendingAssistantOperationsNode = null;
  pendingAssistantThinkingHost = null;
  pendingAssistantTextNode = null;
  pendingAssistantStableNode = null;
  pendingAssistantTailNode = null;
  pendingAssistantThinkingNode = null;
  pendingAssistantStatusTextNode = null;
  pendingAssistantStatusTimeNode = null;
  pendingAssistantRenderedRuntimeText = null;
  pendingAssistantRenderedStableText = null;
  pendingAssistantRenderedTailText = null;
  pendingAssistantRenderedOperationsHtml = null;
  pendingAssistantStoryDirty = false;
  pendingAssistantOperationsDirty = false;
  pendingAssistantThinkingDirty = false;
}

function syncPendingAssistantStatus() {
  const latest = Array.isArray(liveProcessEvents) ? liveProcessEvents[liveProcessEvents.length - 1] : null;
  const runtimeLabel = String(latest?.label || activityLabel || pendingAssistantPlaceholderText()).trim();
  const runtimeTime = activeAssistantTurn?.startedAt
    ? formatElapsedSince(activeAssistantTurn.startedAt)
    : formatActivityDuration();
  if (pendingAssistantStatusTextNode) {
    pendingAssistantStatusTextNode.textContent = runtimeLabel;
    pendingAssistantStatusTextNode.classList.add("is-live");
  }
  if (pendingAssistantStatusTimeNode) {
    pendingAssistantStatusTimeNode.textContent = runtimeTime;
  }
  if (runtimeLabel) {
    activityLabel = runtimeLabel;
    if (!activityStartedAt) {
      activityStartedAt = Date.now();
    }
    updateActivityPill();
  }
}

function latestRuntimeNarration(turn) {
  if (!turn) return "";
  const existingNarration = cleanDisplayText(String(turn.runtimeNarration || "").trim(), "");
  if (existingNarration) {
    return existingNarration;
  }
  const segments = [];
  const pushSegment = (value) => {
    const text = normalizeAgentStageNarration(value);
    if (!text) return;
    if (!segments.includes(text)) {
      segments.push(text);
    }
  };
  const worklog = Array.isArray(turn.worklog) ? turn.worklog : [];
  if (worklog.length) {
    const latest = worklog[worklog.length - 1] || null;
    const previous = worklog.length > 1 ? worklog[worklog.length - 2] : null;
    [previous?.text, latest?.text].forEach(pushSegment);
  }
  if (segments.length) {
    return segments.slice(-2).join(currentLanguage === "zh" ? "\n\n" : "\n\n");
  }
  const latestProcess = Array.isArray(turn.process) && turn.process.length
    ? turn.process[turn.process.length - 1]
    : null;
  if (latestProcess?.detail) {
    pushSegment(latestProcess.detail);
  }
  if (segments.length < 2 && latestProcess?.label) {
    pushSegment(latestProcess.label);
  }
  const latestTool = Array.isArray(turn.tools) && turn.tools.length
    ? turn.tools[turn.tools.length - 1]
    : null;
  if (segments.length < 2) {
    pushSegment(summarizeRuntimeToolNarration(latestTool));
  }
  const latestDiff = Array.isArray(turn.diffs) && turn.diffs.length
    ? turn.diffs[turn.diffs.length - 1]
    : null;
  if (segments.length < 2) {
    pushSegment(summarizeRuntimeDiffNarration(latestDiff));
  }
  return segments.slice(-2).join(currentLanguage === "zh" ? "\n\n" : "\n\n");
}

function latestStreamingNarrationSegments(turn, limit = 3) {
  const segments = [];
  const pushSegment = (value) => {
    const text = summarizeOperationalText(value, "");
    if (!text) return;
    const duplicated = segments.some((existing) =>
      existing === text
      || existing.includes(text)
      || text.includes(existing)
    );
    if (!duplicated) {
      segments.push(text);
    }
  };
  const worklog = Array.isArray(turn?.worklog) ? turn.worklog : [];
  worklog.slice(-Math.max(1, limit)).forEach((entry) => pushSegment(entry?.text));
  return segments;
}

function latestStreamingWorklogNarration(turn, limit = 3) {
  return latestStreamingNarrationSegments(turn, limit).join("\n\n");
}

function inlineStreamingNarrationText(turn, explicitContent = "") {
  if (!turn) return "";
  const explicit = String(explicitContent || "");
  const explicitCore = cleanDisplayText(normalizedAssistantSubstantiveContent(explicit), "");
  const narration = summarizeOperationalText(String(turn.progressNarration || "").trim(), "")
    || latestStreamingWorklogNarration(turn, explicitCore ? 2 : 3)
    || summarizeOperationalText(latestRuntimeNarration(turn), "");
  if (!narration) return "";
  if (!explicitCore) return narration;
  if (
    explicitCore === narration
    || explicitCore.includes(narration)
    || narration.includes(explicitCore)
  ) {
    return "";
  }
  return narration;
}

function composeStreamingAssistantVisibleContent(turn) {
  if (!turn) return "";
  const textSegments = Array.isArray(turn?.textSegments) ? turn.textSegments : [];
  if (textSegments.length) {
    const combined = textSegments
      .map((item) => sanitizeMessageContent(String(item?.text || "")).trim())
      .filter((item) => item && !looksLikeOperationalContentDump(item) && !shouldSuppressInlineAssistantCode(item, turn?.diffs || []))
      .join("\n\n")
      .trim();
    if (combined) return combined;
  }
  const anchored = sanitizeMessageContent(String(turn?.streamingAnchorText || "")).trim();
  if (
    anchored
    && !looksLikeOperationalContentDump(anchored)
    && !assistantTextLooksLikeProcessNarration(anchored)
    && !shouldSuppressInlineAssistantCode(anchored, turn?.diffs || [])
  ) {
    return anchored;
  }
  const explicitContent = String(turn.text || "");
  const cleaned = cleanDisplayText(explicitContent, "").trim();
  if (!cleaned) return "";
  if (looksLikeOperationalContentDump(cleaned)) return "";
  if (assistantTextLooksLikeProcessNarration(cleaned)) return "";
  if (shouldSuppressInlineAssistantCode(cleaned, turn?.diffs || [])) return "";
  return explicitContent;
}

function completionFallbackAssistantContent(turn) {
  const textSegments = Array.isArray(turn?.textSegments) ? turn.textSegments : [];
  if (textSegments.length) {
    return textSegments
      .map((item) => sanitizeMessageContent(String(item?.text || "")).trim())
      .filter((item) => item && !looksLikeOperationalContentDump(item) && !shouldSuppressInlineAssistantCode(item, turn?.diffs || []))
      .join("\n\n")
      .trim();
  }
  const assistantText = sanitizeMessageContent(String(turn?.text || "")).trim();
  if (!assistantText) return "";
  if (looksLikeOperationalContentDump(assistantText)) return "";
  if (assistantTextLooksLikeProcessNarration(assistantText)) return "";
  if (shouldSuppressInlineAssistantCode(assistantText, turn?.diffs || [])) return "";
  return assistantText;
}

function hasRenderableAssistantText(turn) {
  return Boolean(completionFallbackAssistantContent(turn));
}

function messageHasAssistantChoices(message) {
  return Boolean(
    message
    && message.kind === "message"
    && message.role === "assistant"
    && message.assistant_choices
    && Array.isArray(message.assistant_choices.options)
    && message.assistant_choices.options.some((item) => cleanDisplayText(item || "", "")),
  );
}

function turnHasRenderableAssistantContent(turn) {
  if (!turn) return false;
  if (hasRenderableAssistantText(turn)) return true;
  return Boolean(
    turn.assistantChoices
    && Array.isArray(turn.assistantChoices.options)
    && turn.assistantChoices.options.some((item) => cleanDisplayText(item || "", "")),
  );
}

function latestVisibleAssistantTurn(messages) {
  const turns = groupMessagesIntoTurns(visibleConversationMessages(messages || []));
  return [...turns].reverse().find((turn) => turn?.kind === "assistant_turn" && turn?.data)?.data || null;
}

function shouldPreferPersistedAssistantTurn(messages, fallbackTurn = activeAssistantTurn) {
  const fallbackText = cleanDisplayText(completionFallbackAssistantContent(fallbackTurn), "");
  const fallbackCore = cleanDisplayText(assistantPrimaryReplyCore(fallbackText), "");
  const persistedTurn = latestVisibleAssistantTurn(messages);
  const persistedText = cleanDisplayText(completionFallbackAssistantContent(persistedTurn), "");
  const persistedCore = cleanDisplayText(assistantPrimaryReplyCore(persistedText), "");
  if (!turnHasRenderableAssistantContent(persistedTurn)) return false;
  if (!persistedText) {
    return Boolean(
      persistedTurn?.assistantChoices
      && Array.isArray(persistedTurn.assistantChoices.options)
      && persistedTurn.assistantChoices.options.length,
    );
  }
  if (!fallbackText) return true;
  if (persistedText === fallbackText) return true;
  if (persistedText.includes(fallbackText)) return true;
  if (fallbackCore && persistedCore && (persistedCore === fallbackCore || persistedCore.includes(fallbackCore) || fallbackCore.includes(persistedCore))) {
    return true;
  }
  if (looksLikeStructuredAssistantReport(persistedText) && !looksLikeStructuredAssistantReport(fallbackText)) {
    return true;
  }
  return false;
}

function reconcileVisibleSessionCompletion(nextData, { sessionId = null, preserveScroll = true } = {}) {
  const targetSessionId = String(sessionId || nextData?.current_session_id || bootstrapData?.current_session_id || "").trim();
  if (!targetSessionId) return false;
  const visibleSessionId = String(bootstrapData?.current_session_id || "").trim();
  const isVisibleSession = visibleSessionId && visibleSessionId === targetSessionId;
  const runtimeEnded = !bootstrapHasLiveSession(nextData, targetSessionId);
  if (!isVisibleSession || !runtimeEnded) {
    return false;
  }

  const currentState = getSessionRunState(targetSessionId);
  if (!currentState?.running && !pendingAssistantBubble) {
    return false;
  }

  const incomingMessages = visibleConversationMessages(nextData?.messages || []);
  const preferPersistedAssistant = shouldPreferPersistedAssistantTurn(incomingMessages);
  endSessionRun(targetSessionId);

  if (incomingMessages.length) {
    bootstrapData = {
      ...(bootstrapData || {}),
      ...nextData,
      messages: incomingMessages,
      current_session_id: targetSessionId,
    };
    renderReview(buildReviewFromMessages(incomingMessages));
    syncAgentPreludeBackground(incomingMessages);
    if (pendingAssistantBubble) {
      const finalizedInPlace = finalizeVisibleAssistantBubble(incomingMessages);
      if (!finalizedInPlace) {
        renderMessages(incomingMessages, { preserveScroll });
      }
    } else {
      renderMessages(incomingMessages, { preserveScroll });
    }
  }

  if (!preferPersistedAssistant && turnHasRenderableAssistantContent(activeAssistantTurn)) {
    materializePendingConversationMessages({ sessionId: targetSessionId });
  }

  finalizeActiveAssistantTurn();
  pendingPermissionRequest = null;
  liveToolEvents = [];
  liveEditedFiles = [];
  liveProcessEvents = [];
  renderAgentRuntimeStrip();
  renderAgentProcessStrip();
  renderPermissionStrip();
  return true;
}

function visibleStreamingRuntimeNarration(turn) {
  if (!turn) return "";
  const narration = structureAssistantDisplayText(latestRuntimeNarration(turn));
  if (!narration) return "";
  const composed = structureAssistantDisplayText(composeStreamingAssistantVisibleContent(turn));
  const normalizedComposed = cleanDisplayText(composed, "");
  const normalizedNarration = cleanDisplayText(narration, "");
  if (normalizedComposed) {
    if (
      normalizedComposed === normalizedNarration
      || normalizedComposed.includes(normalizedNarration)
      || normalizedNarration.includes(normalizedComposed)
    ) {
      return "";
    }
  }
  const inlineNarration = structureAssistantDisplayText(inlineStreamingNarrationText(turn, String(turn.text || "")));
  const normalizedInline = cleanDisplayText(inlineNarration, "");
  if (normalizedInline) {
    if (
      normalizedInline === normalizedNarration
      || normalizedInline.includes(normalizedNarration)
      || normalizedNarration.includes(normalizedInline)
    ) {
      return "";
    }
  }
  const explicit = structureAssistantDisplayText(String(turn.text || ""));
  const normalizedExplicit = cleanDisplayText(explicit, "");
  if (!normalizedExplicit) return normalizedNarration;
  if (
    normalizedNarration === normalizedExplicit
    || normalizedExplicit.includes(normalizedNarration)
    || normalizedNarration.includes(normalizedExplicit)
  ) {
    return "";
  }
  return normalizedNarration;
}

function syncPendingAssistantText() {
  if (!pendingAssistantBubble || !activeAssistantTurn) return;
  if (!ensurePendingAssistantShellBound()) return;
  const content = composeStreamingAssistantVisibleContent(activeAssistantTurn);
  const shouldSuppressInlineCode = shouldSuppressInlineAssistantCode(
    content,
    activeAssistantTurn?.diffs || [],
  );
  if (shouldSuppressInlineCode) {
    activeAssistantTurn.suppressedInlineContent = true;
  }
  const visibleContent = shouldSuppressInlineCode || (!content && activeAssistantTurn.suppressedInlineContent)
    ? visibleAssistantWorkspaceNotice(activeAssistantTurn)
    : content;
  if (pendingAssistantStoryNode) {
    const streamingParts = renderStreamingAssistantContent(visibleContent, {
      placeholder: pendingAssistantPlaceholderText(),
      streaming: true,
    });
    const newStoryHtml = renderTurnStoryline(activeAssistantTurn, {
      streaming: true,
      streamingParts,
      fallbackText: visibleContent,
      overrideText: shouldSuppressInlineCode ? visibleContent : "",
    });
    if (pendingAssistantStoryNode.innerHTML !== newStoryHtml) {
      pendingAssistantStoryNode.innerHTML = newStoryHtml;
      bindPendingAssistantStoryNodes(pendingAssistantStoryNode);
    }
  }
  if (!pendingAssistantTextNode) return;
  const parts = renderStreamingAssistantContent(visibleContent, {
    placeholder: pendingAssistantPlaceholderText(),
    streaming: true,
  });
  if (pendingAssistantStableNode || pendingAssistantTailNode) {
    if (pendingAssistantStableNode && pendingAssistantRenderedStableText !== parts.stableHtml) {
      pendingAssistantStableNode.innerHTML = parts.stableHtml || "";
      pendingAssistantRenderedStableText = parts.stableHtml || "";
    }
    if (pendingAssistantTailNode && pendingAssistantRenderedTailText !== parts.tailText) {
      pendingAssistantTailNode.textContent = parts.tailText || "";
      pendingAssistantTailNode.hidden = !parts.tailText;
      pendingAssistantRenderedTailText = parts.tailText || "";
    }
  } else if (pendingAssistantRenderedStableText !== parts.html) {
    pendingAssistantTextNode.innerHTML = parts.html;
    pendingAssistantRenderedStableText = parts.html;
  }
  if (pendingAssistantRuntimeStatusesNode) {
    const newHtml = "";
    if (pendingAssistantRenderedRuntimeText !== newHtml) {
      pendingAssistantRuntimeStatusesNode.innerHTML = newHtml;
      pendingAssistantRenderedRuntimeText = newHtml;
    }
    pendingAssistantRuntimeStatusesNode.hidden = !newHtml;
  }
  syncPendingAssistantOperations();
  syncPendingAssistantRuntimePanel();
  pendingAssistantBubble?.querySelector(".codex-answer")?.classList.toggle("codex-answer-empty", parts.isEmpty);
}

function schedulePendingAssistantTextSync({ keepBottom = false } = {}) {
  if (pendingAssistantTextFrame != null) return;
  pendingAssistantTextFrame = window.requestAnimationFrame(() => {
    pendingAssistantTextFrame = null;
    syncPendingAssistantText();
    syncPendingThinkingIndicator();
    syncPendingAssistantStatus();
    if (keepBottom) {
      scrollMessageStreamToBottom(true);
    }
  });
}

function schedulePendingAssistantStatusSync() {
  if (pendingAssistantStatusFrame != null) return;
  pendingAssistantStatusFrame = window.requestAnimationFrame(() => {
    pendingAssistantStatusFrame = null;
    if (!pendingAssistantBubble || !activeAssistantTurn) return;
    syncPendingAssistantStatus();
  });
}

function refreshPendingAssistantBubble() {
  if (!pendingAssistantBubble || !activeAssistantTurn) return;
  if (!ensurePendingAssistantShellBound()) return;

  // If only thinking changed, update thinking content in-place — avoid full re-render
  if (pendingAssistantThinkingDirty) {
    pendingAssistantThinkingDirty = false;
    syncPendingThinkingIndicator();
  }

  if (pendingAssistantOperationsDirty) {
    pendingAssistantOperationsDirty = false;
    syncPendingAssistantOperations();
    syncPendingAssistantRuntimePanel();
  }

  if (pendingAssistantStoryDirty) {
    pendingAssistantStoryDirty = false;
    if (pendingAssistantStoryNode) {
      const keepBottom = isNearMessageStreamBottom();
      syncPendingAssistantText();
      syncPendingThinkingIndicator();
      syncPendingAssistantStatus();
      ensurePendingThinkingExpanded();
      if (keepBottom) {
        scrollMessageStreamToBottom(true);
      }
      return;
    }
    pendingAssistantTextNode = null;
    pendingAssistantStableNode = null;
    pendingAssistantTailNode = null;
    pendingAssistantRuntimeStatusesNode = null;
    pendingAssistantOperationsNode = null;
    pendingAssistantRenderedRuntimeText = null;
    pendingAssistantRenderedStableText = null;
    pendingAssistantRenderedTailText = null;
    pendingAssistantRenderedOperationsHtml = null;
  }

  if (pendingAssistantTextNode) {
    const keepBottom = isNearMessageStreamBottom();
    syncPendingAssistantText();
    syncPendingThinkingIndicator();
    syncPendingAssistantStatus();
    ensurePendingThinkingExpanded();
    if (keepBottom) {
      scrollMessageStreamToBottom(true);
    }
    return;
  }
  if (pendingAssistantBubbleFrame != null) return;
  pendingAssistantBubbleFrame = window.requestAnimationFrame(() => {
    pendingAssistantBubbleFrame = null;
    if (!pendingAssistantBubble || !activeAssistantTurn) return;
    const keepBottom = isNearMessageStreamBottom();
    if (!ensurePendingAssistantShellBound()) return;
    syncPendingAssistantText();
    syncPendingThinkingIndicator();
    syncPendingAssistantStatus();
    ensurePendingThinkingExpanded();
    if (keepBottom) {
      scrollMessageStreamToBottom(true);
    }
  });
}

function findToolEntry(callId) {
  if (!activeAssistantTurn || !callId) return null;
  return activeAssistantTurn.tools.find((item) => item.call_id === callId) || null;
}

function upsertToolEntry(tool) {
  if (!activeAssistantTurn || !tool) return;
  const existing = findToolEntry(tool.call_id);
  if (existing) {
    Object.assign(existing, tool);
    return;
  }
  activeAssistantTurn.tools.push({
    call_id: tool.call_id || `${Date.now()}`,
    name: tool.name || "tool",
    status: tool.status || "pending",
    risk: tool.risk || "",
    args: tool.args || null,
    file_path: tool.file_path || "",
    result: tool.result || "",
    success: tool.success ?? null,
  });
}

function upsertDiffEntry(diff) {
  if (!activeAssistantTurn || !diff?.path) return;
  const existing = activeAssistantTurn.diffs.find((item) => item.path === diff.path);
  if (existing) {
    const changed =
      Number(existing.added || 0) !== Number(diff.added || 0) ||
      Number(existing.removed || 0) !== Number(diff.removed || 0) ||
      String(existing.before_content || "") !== String(diff.before_content || "") ||
      String(existing.after_content || "") !== String(diff.after_content || "");
    existing.added = diff.added || 0;
    existing.removed = diff.removed || 0;
    existing.before_content = String(diff.before_content || existing.before_content || "");
    existing.after_content = String(diff.after_content || existing.after_content || "");
    existing.updated_at = Date.now();
    if (changed) {
      setDiffReviewStatus(existing, "pending");
    }
    return;
  }
  const nextDiff = {
    path: diff.path,
    added: diff.added || 0,
    removed: diff.removed || 0,
    before_content: String(diff.before_content || ""),
    after_content: String(diff.after_content || ""),
    updated_at: Date.now(),
  };
  activeAssistantTurn.diffs.push(nextDiff);
  setDiffReviewStatus(nextDiff, "pending");
}

function toolStatusLabel(status) {
  const key = String(status || "").toLowerCase();
  const labels = {
    pending: currentLanguage === "zh" ? "排队中" : "Queued",
    approved: currentLanguage === "zh" ? "已批准" : "Approved",
    denied: currentLanguage === "zh" ? "已拒绝" : "Denied",
    executing: currentLanguage === "zh" ? "执行中" : "Running",
    complete: currentLanguage === "zh" ? "完成" : "Done",
    failed: currentLanguage === "zh" ? "失败" : "Failed",
  };
  return labels[key] || status || "Running";
}

function compactToolArgs(args) {
  if (!args) return "";
  try {
    const raw = JSON.stringify(args);
    return raw.length > 88 ? `${raw.slice(0, 85)}...` : raw;
  } catch (_error) {
    return "";
  }
}

function toolStatusVerb(status) {
  const key = String(status || "").toLowerCase();
  const labels = {
    pending: currentLanguage === "zh" ? "准备工具调用" : "Preparing tool call",
    approved: currentLanguage === "zh" ? "工具已批准" : "Tool approved",
    denied: currentLanguage === "zh" ? "工具已拒绝" : "Tool denied",
    executing: currentLanguage === "zh" ? "执行中" : "Running",
    failed: currentLanguage === "zh" ? "执行失败" : "Failed",
  };
  return labels[key] || (currentLanguage === "zh" ? "工具调用" : "Tool call");
}

function buildRuntimeContextDetailForSync(turn) {
  if (!turn) return "";
  const tools = Array.isArray(turn.tools) ? turn.tools : [];
  const latest = tools[tools.length - 1];
  if (!latest?.name) return "";
  const fileParam = (latest.params?.file_path || latest.params?.path || latest.params?.target_file || "").trim();
  if (fileParam) {
    const parts = fileParam.replace(/\\/g, "/").split("/");
    return parts[parts.length - 1];
  }
  return "";
}

function renderWorklogRuntimeStatuses(turn, isStreaming) {
  if (!turn) return "";
  if (!autoOpenActivityPanel) return "";
  const tools = Array.isArray(turn.tools) ? turn.tools : [];
  const diffs = Array.isArray(turn.diffs) ? turn.diffs : [];

  const scanToolNames = new Set(["list_dir", "find_files", "read_file", "read_file_range", "tree_dir", "search_files"]);

  function isScanTool(tool) {
    return scanToolNames.has(String(tool.name || "").toLowerCase());
  }

  function toolStatusClass(tool) {
    const status = String(tool.status || "").toLowerCase();
    if (isStreaming && ["pending", "approved", "executing"].includes(status)) return "run";
    if (status === "failed" || status === "error") return "fail";
    return "done";
  }

  function classFor(state) {
    if (state === "run") return "codex-streaming-runtime--active";
    if (state === "fail") return "codex-streaming-runtime--failed";
    return "codex-streaming-runtime--done";
  }

  const statusEntries = [];

  // Meaningful (non-scan) tools → individual divs
  const meaningfulTools = tools.filter((t) => !isScanTool(t) && summarizeRuntimeToolNarration(t));
  for (const tool of meaningfulTools) {
    const narration = summarizeRuntimeToolNarration(tool);
    const state = toolStatusClass(tool);
    const fileParam = (tool.params?.file_path || tool.params?.path || tool.params?.target_file || tool.file_path || "").trim();
    const detail = fileParam ? displayFileNameOnly(fileParam) : "";
    statusEntries.push({ narration, state, detail, callId: tool.call_id || `tool:${tool.name}` });
  }

  // Scan tools → collapse into at most one active entry + one summary
  const scanTools = tools.filter((t) => isScanTool(t) && summarizeRuntimeToolNarration(t));
  if (scanTools.length) {
    const activeScan = scanTools.find((t) => ["pending", "approved", "executing"].includes(String(t.status || "").toLowerCase()));
    const hasCompleted = scanTools.some((t) => {
      const s = String(t.status || "").toLowerCase();
      return s !== "pending" && s !== "approved" && s !== "executing" && s !== "failed";
    });
    if (activeScan) {
      const fileParam = (activeScan.params?.file_path || activeScan.params?.path || activeScan.params?.target_file || activeScan.file_path || "").trim();
      const detail = fileParam ? displayFileNameOnly(fileParam) : "";
      statusEntries.push({ narration: "正在查看...", state: "run", detail, callId: "scan:active" });
    } else if (hasCompleted) {
      statusEntries.push({ narration: currentLanguage === "zh" ? "已完成查看" : "Inspection done", state: "done", detail: "", callId: "scan:done" });
    }
  }

  // Diffs
  for (const diff of diffs) {
    const narration = summarizeRuntimeDiffNarration(diff);
    if (!narration) continue;
    statusEntries.push({ narration, state: "done", detail: "", callId: `diff:${diff.path}:${Number(diff.added || 0)}:${Number(diff.removed || 0)}` });
  }

  if (!statusEntries.length) return "";

  // Deduplicate by callId
  const seen = new Set();
  const unique = [];
  for (let i = statusEntries.length - 1; i >= 0; i--) {
    if (!seen.has(statusEntries[i].callId)) {
      seen.add(statusEntries[i].callId);
      unique.unshift(statusEntries[i]);
    }
  }
  if (!unique.length) return "";

  const lastIndex = unique.length - 1;
  return unique
    .map((entry, index) => {
      const isLatest = index === lastIndex;
      const dataAttr = isLatest ? ' data-streaming-runtime="latest"' : "";
      return `<div class="codex-streaming-runtime ${classFor(entry.state)}"${dataAttr}>${runtimeStatusLabelHtml(entry.narration, entry.detail)}</div>`;
    })
    .join("");
}

function runtimeStatusLabelHtml(narration, detail) {
  return `<span class="codex-runtime-label">${escapeHtml(narration)}</span>${detail ? `<span class="codex-runtime-detail">${escapeHtml(detail)}</span>` : ""}`;
}

function renderStreamMoment(moment) {
  if (!moment) return "";
  const text = cleanDisplayText(moment.text || "", "");
  if (!text) return "";
  const state = String(moment.state || "").trim().toLowerCase();
  const kind = String(moment.kind || "note").trim().toLowerCase();
  const detail = cleanDisplayText(moment.detail || "", "");
  const filePath = cleanDisplayText(moment.filePath || "", "");
  const fileName = filePath ? displayFileNameOnly(filePath) : "";
  const added = Number(moment.added || 0) || 0;
  const removed = Number(moment.removed || 0) || 0;
  const isEditing = kind === "edit";
  const isCommand = kind === "command";
  const isRunning = state === "run";
  if (kind === "compaction") {
    return `
      <div class="codex-context-compaction${isRunning ? " is-streaming" : ""}">
        <span class="codex-context-compaction-line" aria-hidden="true"></span>
        <span class="codex-context-compaction-text">${escapeHtml(text)}</span>
        <span class="codex-context-compaction-line" aria-hidden="true"></span>
      </div>
    `;
  }
  const prefix = isEditing
    ? zhLabel("已编辑", "Edited")
    : isCommand
      ? zhLabel("已运行", "Ran")
      : "";
  if (isEditing && fileName) {
    const animationOffset = "";
    const editPrefix = isRunning
      ? zhLabel("\u6b63\u5728\u7f16\u8f91", "Editing")
      : text || zhLabel("\u7f16\u8f91\u5b8c\u6210", "Edit done");
    return `
      <div class="codex-inline-moment codex-inline-moment-edit${isRunning ? " is-running" : ""}">
        <span class="codex-inline-moment-icon" aria-hidden="true">
          <svg viewBox="0 0 24 24" focusable="false" aria-hidden="true">
            <path d="M4 20l3.6-.7L18.3 8.6a1.8 1.8 0 0 0 0-2.6l-.3-.3a1.8 1.8 0 0 0-2.6 0L4.7 16.4 4 20z"></path>
            <path d="M13.8 7.3l2.9 2.9"></path>
          </svg>
        </span>
        <span class="codex-inline-moment-prefix${isRunning ? " is-streaming" : ""}" data-text="${escapeHtml(zhLabel("正在编辑", "Editing"))}"${isRunning ? animationOffset : ""}>${escapeHtml(zhLabel("正在编辑", "Editing"))}</span>
        <span class="codex-inline-moment-file">${escapeHtml(fileName)}</span>
        <span class="codex-inline-moment-stats">
          <span class="is-added">+${escapeHtml(String(added))}</span>
          <span class="is-removed">-${escapeHtml(String(removed))}</span>
        </span>
      </div>
    `;
  }
  return `
    <div class="codex-inline-moment codex-inline-moment-status codex-inline-moment-${escapeHtml(state || "done")} codex-inline-moment-${escapeHtml(kind)}">
      <span class="codex-inline-moment-icon" aria-hidden="true">↳</span>
      <span class="codex-inline-moment-text${isRunning ? " is-streaming" : ""}" data-text="${escapeHtml(text)}">${escapeHtml(text)}</span>
      ${detail ? `<span class="codex-inline-moment-detail">${escapeHtml(detail)}</span>` : ""}
      ${!detail && prefix ? `<span class="codex-inline-moment-prefix-static">${escapeHtml(prefix)}</span>` : ""}
    </div>
  `;
}

function buildTurnStoryEntries(turn, options = {}) {
  const fallbackText = sanitizeMessageContent(String(options.fallbackText || "")).trim();
  const overrideText = sanitizeMessageContent(String(options.overrideText || "")).trim();
  const textSegments = Array.isArray(turn?.textSegments) ? turn.textSegments.slice(-10) : [];
  const entries = [];

  if (overrideText) {
    const overrideMoment = extractAssistantOperationalMoment(overrideText, {
      timestamp: Number(turn?.textUpdatedAt || turn?.startedAt || Date.now()) || Date.now(),
    });
    if (overrideMoment) {
      entries.push({
        kind: "moment",
        id: "override-text-moment",
        moment: overrideMoment,
        timestamp: Number(overrideMoment.timestamp || turn?.textUpdatedAt || turn?.startedAt || Date.now()) || Date.now(),
        order: 0,
      });
    } else {
      entries.push({
        kind: "text",
        id: "override-text",
        text: overrideText,
        timestamp: Number(turn?.textUpdatedAt || turn?.startedAt || Date.now()) || Date.now(),
        order: 0,
        isLatest: true,
      });
    }
  } else {
    textSegments.forEach((segment, index) => {
      const text = sanitizeMessageContent(String(segment?.text || "")).trim();
      if (!text || looksLikeOperationalContentDump(text)) return;
      const derivedMoment = extractAssistantOperationalMoment(text, {
        timestamp: Number(segment?.timestamp || 0) || 0,
      });
      if (derivedMoment) {
        entries.push({
          kind: "moment",
          id: `${segment?.id || `text-${index}`}-moment`,
          moment: derivedMoment,
          timestamp: Number(derivedMoment.timestamp || segment?.timestamp || 0) || 0,
          order: index,
        });
        return;
      }
      entries.push({
        kind: "text",
        id: segment?.id || `text-${index}`,
        text,
        timestamp: Number(segment?.timestamp || 0) || 0,
        order: index,
        isLatest: index === textSegments.length - 1,
      });
    });

    if (!entries.length && fallbackText && !looksLikeOperationalContentDump(fallbackText)) {
      const fallbackMoment = extractAssistantOperationalMoment(fallbackText, {
        timestamp: Number(turn?.textUpdatedAt || turn?.startedAt || Date.now()) || Date.now(),
      });
      if (fallbackMoment) {
        entries.push({
          kind: "moment",
          id: "fallback-text-moment",
          moment: fallbackMoment,
          timestamp: Number(fallbackMoment.timestamp || turn?.textUpdatedAt || turn?.startedAt || Date.now()) || Date.now(),
          order: 0,
        });
      } else {
        entries.push({
          kind: "text",
          id: "fallback-text",
          text: fallbackText,
          timestamp: Number(turn?.textUpdatedAt || turn?.startedAt || Date.now()) || Date.now(),
          order: 0,
          isLatest: true,
        });
      }
    }

    const streamMoments = Array.isArray(turn?.streamMoments) ? turn.streamMoments.slice(-12) : [];
    streamMoments.forEach((moment, index) => {
      if (!cleanDisplayText(moment?.text || "", "") || looksLikeOperationalContentDump(moment?.text || "")) return;
      entries.push({
        kind: "moment",
        id: moment?.id || `moment-${index}`,
        moment,
        timestamp: Number(moment?.timestamp || 0) || Number(turn?.startedAt || Date.now()),
        order: textSegments.length + index,
      });
    });
  }

  return entries.sort((left, right) => {
    const leftTime = Number(left.timestamp || 0) || 0;
    const rightTime = Number(right.timestamp || 0) || 0;
    if (leftTime !== rightTime) return leftTime - rightTime;
    if (left.kind !== right.kind) return left.kind === "text" ? -1 : 1;
    return Number(left.order || 0) - Number(right.order || 0);
  });
}

function renderTurnStoryTextEntry(entry, options = {}) {
  if (!entry?.text) return "";
  if (options.streaming && entry.isLatest && options.streamingParts) {
    return `
      <div class="codex-stream-story-text codex-stream-story-text-live">
        <div class="codex-streaming-text markdown-body" data-streaming-markdown>
          <div data-streaming-markdown-stable>${options.streamingParts?.stableHtml || ""}</div>
          <pre class="codex-streaming-tail"${options.streamingParts?.tailText ? "" : " hidden"} data-streaming-markdown-tail>${escapeHtml(options.streamingParts?.tailText || "")}</pre>
        </div>
      </div>
    `;
  }
  return `<div class="codex-stream-story-text markdown-body">${renderMarkdown(structureAssistantDisplayText(entry.text || ""))}</div>`;
}

function renderTurnStoryline(turn, options = {}) {
  const streaming = Boolean(options.streaming);
  const entries = buildTurnStoryEntries(turn, {
    fallbackText: options.fallbackText || "",
    overrideText: options.overrideText || "",
  });
  if (!entries.length && streaming && options.streamingParts) {
    return `
      <div class="codex-stream-story" data-turn-storyline>
        ${renderTurnStoryTextEntry(
          { text: options.fallbackText || "", isLatest: true },
          { streaming: true, streamingParts: options.streamingParts },
        )}
      </div>
    `;
  }
  if (!entries.length) return "";
  return `
    <div class="codex-stream-story" data-turn-storyline>
      ${entries.map((entry) => entry.kind === "moment"
        ? renderStreamMoment(entry.moment)
        : renderTurnStoryTextEntry(entry, {
            streaming,
            streamingParts: streaming && entry.isLatest ? options.streamingParts : null,
          })).filter(Boolean).join("")}
    </div>
  `;
}

function renderTurnOperations(turn) {
  const sourceMoments = Array.isArray(turn?.streamMoments)
    ? turn.streamMoments.filter((moment) => cleanDisplayText(moment?.text || "", "") && !looksLikeOperationalContentDump(moment?.text || ""))
    : [];
  const groupedKinds = new Set(["inspection", "edit", "command", "check", "tool"]);
  const latestByKind = new Map();
  const passthrough = [];
  sourceMoments.forEach((moment) => {
    const kind = String(moment?.kind || "").toLowerCase();
    if (groupedKinds.has(kind)) {
      latestByKind.set(kind, moment);
    } else {
      passthrough.push(moment);
    }
  });
  const moments = [...passthrough, ...latestByKind.values()]
    .sort((left, right) => (Number(left?.timestamp || 0) || 0) - (Number(right?.timestamp || 0) || 0))
    .slice(-12);
  return moments.map((moment) => renderStreamMoment(moment)).filter(Boolean).join("");
}

function summarizeRuntimeToolNarration(tool) {
  if (!tool) return "";
  const name = cleanDisplayText(tool?.name || "", "");
  if (!name) return "";
  const status = String(tool?.status || "").toLowerCase();
  const workspaceScanTools = new Set([
    "list_dir",
    "find_files",
    "read_file",
    "read_file_range",
    "tree_dir",
    "search_files",
  ]);
  const editTools = new Set([
    "write_file",
    "apply_patch",
    "search_and_replace",
    "search_and_replace_multi",
    "rename_path",
    "mkdir",
  ]);
  const gitLike = name.startsWith("git_");
  const isWorkspaceScan = workspaceScanTools.has(name);
  const isEdit = editTools.has(name);
  if (currentLanguage === "zh") {
    if (status === "failed") {
      if (isWorkspaceScan) return "查看失败";
      if (isEdit) return "编辑失败";
      if (gitLike) return "检查失败";
      return "执行失败";
    }
    if (["pending", "approved", "executing"].includes(status)) {
      if (isWorkspaceScan) return "正在查看...";
      if (isEdit) return "正在编辑...";
      if (gitLike) return "正在检查...";
      return "正在执行...";
    }
    if (isWorkspaceScan) return "已完成查看";
    if (isEdit) return "已完成编辑";
    if (gitLike) return "已完成检查";
    return "已完成";
  }
  if (status === "failed") {
    if (isWorkspaceScan) return "Inspect failed";
    if (isEdit) return "Edit failed";
    if (gitLike) return "Check failed";
    return "Failed";
  }
  if (["pending", "approved", "executing"].includes(status)) {
    if (isWorkspaceScan) return "Inspecting...";
    if (isEdit) return "Editing...";
    if (gitLike) return "Checking...";
    return "Running...";
  }
  if (isWorkspaceScan) return "Inspection done";
  if (isEdit) return "Edit done";
  if (gitLike) return "Check done";
  return "Done";
}

function summarizeRuntimeDiffNarration(diff) {
  if (!diff?.path) return "";
  const path = displayFileNameOnly(diff.path || "");
  const added = Number(diff.added || 0);
  const removed = Number(diff.removed || 0);
  if (currentLanguage === "zh") {
    return `已更新 ${path} (+${added} / -${removed})`;
  }
  return `Updated ${path} (+${added} / -${removed})`;
}

function summarizeCompletedTurnResult(turn) {
  if (!turn) return { narration: "", hasFailure: false };
  const tools = Array.isArray(turn.tools) ? turn.tools : [];
  const diffs = Array.isArray(turn.diffs) ? turn.diffs : [];

  const completedTools = tools.filter((t) => {
    const status = String(t?.status || "").toLowerCase();
    return status === "completed" || status === "success" || status === "ok" || status === "";
  });
  const failedTools = tools.filter((t) => {
    const status = String(t?.status || "").toLowerCase();
    return status === "failed" || status === "error";
  });

  if (failedTools.length > 0) {
    return { narration: currentLanguage === "zh" ? "部分执行失败" : "Partial failure", hasFailure: true };
  }

  if (completedTools.length > 0 || diffs.length > 0) {
    const narratives = completedTools.map(summarizeRuntimeToolNarration).filter(Boolean);
    const diffNarratives = diffs.map(summarizeRuntimeDiffNarration).filter(Boolean);
    const allNarratives = [...new Set([...narratives, ...diffNarratives])];
    if (allNarratives.length) {
      return { narration: allNarratives.slice(-2).join(" · "), hasFailure: false };
    }
  }

  if (tools.length > 0) {
    return { narration: currentLanguage === "zh" ? "已完成" : "Done", hasFailure: false };
  }

  return { narration: currentLanguage === "zh" ? "已完成" : "Done", hasFailure: false };
}

function isCommandLikeTool(name) {
  const normalized = String(name || "").trim().toLowerCase();
  return [
    "run_command",
    "run_safe_command",
    "run_python",
    "run_python_file",
    "run_r",
    "run_julia",
    "terminal_run",
  ].includes(normalized);
}

function summarizeTurnRuntime(turn) {
  const tools = Array.isArray(turn?.tools) ? turn.tools : [];
  const diffs = Array.isArray(turn?.diffs) ? turn.diffs : [];
  const subagents = dedupeSubagentEntries(turn?.subagents);
  const verifier = turn?.verifierReport || null;
  const commandCount = tools.filter((tool) => isCommandLikeTool(tool?.name)).length;
  const totalTools = tools.length;
  const changedFiles = diffs.length;
  const verifierChecks = Array.isArray(verifier?.checks) ? verifier.checks.length : 0;
  const parts = [];
  if (totalTools > 0) {
    parts.push(currentLanguage === "zh" ? `${totalTools} 个工具` : `${totalTools} tools`);
  }
  if (commandCount > 0) {
    parts.push(currentLanguage === "zh" ? `${commandCount} 条命令` : `${commandCount} commands`);
  }
  if (changedFiles > 0) {
    parts.push(currentLanguage === "zh" ? `${changedFiles} 个文件已修改` : `${changedFiles} files changed`);
  }
  if (subagents.length > 0) {
    parts.push(currentLanguage === "zh" ? `${subagents.length} 个子代理` : `${subagents.length} subagents`);
  }
  if (verifierChecks > 0) {
    parts.push(currentLanguage === "zh" ? `${verifierChecks} 项检查` : `${verifierChecks} checks`);
  }
  return parts;
}

function renderAssistantTurn(turn, options = {}) {
  const runtimeLabel = String(
    (Array.isArray(turn?.process) && turn.process.length
      ? turn.process[turn.process.length - 1]?.label
      : "") || turn?.activity || pendingAssistantPlaceholderText(),
  ).trim();
  const runtimeTime = turn?.startedAt ? formatElapsedSince(turn.startedAt) : "";
  const process = Array.isArray(turn?.process) ? turn.process : [];
  const processDelegates = Array.isArray(turn?.processDelegates) ? turn.processDelegates : [];
  const subagents = dedupeSubagentEntries(turn?.subagents);
  const verifierReport = turn?.verifierReport || null;
  const runtimeCheckpoints = Array.isArray(turn?.runtimeCheckpoints) ? turn.runtimeCheckpoints : [];
  const branchNotes = Array.isArray(turn?.branchNotes) ? turn.branchNotes : [];
  const timeline = Array.isArray(turn?.timeline) ? turn.timeline : [];
  const tools = Array.isArray(turn?.tools) ? turn.tools : [];
  const diffs = Array.isArray(turn?.diffs) ? turn.diffs : [];
  const thinking = Array.isArray(turn?.thinking) ? turn.thinking : [];
  const permission = turn?.permission || null;
  const text = turn?.text || "";
  const cleanedText = displayMarkdownText(text);
  const isStreaming = Boolean(options.streaming);
  const showStreamingThinking = Boolean(
    isStreaming
    && turn?.isThinkingPhase,
  );
  const decisionPresentation = !isStreaming
    ? (
      shouldRenderAssistantDecisionCard(turn, cleanedText)
        ? {
            body: cleanedText,
            card: {
              title: cleanDisplayText(turn.assistantChoices.title || "", zhLabel("选择下一步", "Choose next step")) || zhLabel("选择下一步", "Choose next step"),
              options: turn.assistantChoices.options.slice(),
            },
          }
        : { body: cleanedText, card: null }
    )
    : { body: "", card: null };
  const runtimeSummaryParts = summarizeTurnRuntime(turn);
  const hasRuntimeArtifacts = Boolean(
    process.length
    || processDelegates.length
    || subagents.length
    || verifierReport
    || runtimeCheckpoints.length
    || branchNotes.length
    || timeline.length
    || tools.length
    || diffs.length
    || (Array.isArray(turn?.worklog) && turn.worklog.length),
  );
  const streamingInlineRuntime = isStreaming || hasRuntimeArtifacts;
  const inlineResearchDelegateDetails = currentWorkspaceMode !== "research";
  const visibleProcess = streamingInlineRuntime
    ? (isStreaming ? process.slice(-1) : process.slice(-3))
    : [];
  const visibleTools = streamingInlineRuntime
    ? (isStreaming ? tools.slice(-3) : tools.slice(-4))
    : [];
  const visibleDiffs = diffs;
  const showRuntimeHead = false;
  const fallbackNarration = latestRuntimeNarration(turn);
  const runtimeLeadText = "";
  const displayedText = isStreaming
    ? composeStreamingAssistantVisibleContent(turn)
    : (decisionPresentation.body || cleanedText.trim());
  const shouldSuppressInlineCode = shouldSuppressInlineAssistantCode(displayedText, diffs);
  const visibleText = shouldSuppressInlineCode
    ? visibleAssistantWorkspaceNotice(turn)
    : structureAssistantDisplayText(displayedText);

  const runtimeHead = showRuntimeHead
    ? `
      <div class="codex-turn-head"${runtimeLabel ? "" : " hidden"}>
        <div class="codex-turn-status">
          <span class="codex-turn-status-dot" aria-hidden="true"></span>
          <span class="codex-turn-status-text" data-turn-activity>${escapeHtml(runtimeLabel)}</span>
          <span class="codex-turn-status-time" data-turn-elapsed>${escapeHtml(runtimeTime)}</span>
        </div>
      </div>
    `
    : "";

  const runtimeSummaryMarkup = runtimeSummaryParts.length
    ? `
      <div class="codex-runtime-summary" aria-label="${escapeHtml(currentLanguage === "zh" ? "运行摘要" : "Runtime summary")}">
        ${runtimeSummaryParts
          .map((part) => `<span class="codex-runtime-chip">${escapeHtml(part)}</span>`)
          .join("")}
      </div>
    `
    : "";

  const processMarkup = "";

  const delegateMarkup = streamingInlineRuntime && processDelegates.length
    ? `
      <div class="codex-delegate-list codex-steps-list">
        ${processDelegates
          .map((delegate, index) => `
            <details class="codex-delegate-card"${isStreaming ? (index === 0 ? " open" : "") : ""}>
              <summary class="codex-delegate-summary">
                <span class="codex-delegate-name">${escapeHtml(delegate.name || "delegate")}</span>
                <span class="codex-delegate-pill">${escapeHtml(renderDelegateStatus(delegate.status || ""))}</span>
              </summary>
              <div class="codex-delegate-body">
                ${delegate.purpose ? `<div class="codex-delegate-line"><span class="codex-delegate-key">${escapeHtml(zhLabel("目的", "Purpose"))}</span><span class="codex-delegate-value">${escapeHtml(delegate.purpose)}</span></div>` : ""}
                ${delegate.input ? `<div class="codex-delegate-line"><span class="codex-delegate-key">${escapeHtml(zhLabel("输入", "Input"))}</span><span class="codex-delegate-value">${escapeHtml(delegate.input)}</span></div>` : ""}
                ${delegate.output ? `<div class="codex-delegate-line"><span class="codex-delegate-key">${escapeHtml(zhLabel("输出", "Output"))}</span><span class="codex-delegate-value">${escapeHtml(delegate.output)}</span></div>` : ""}
              </div>
            </details>
          `)
          .join("")}
      </div>
    `
    : "";

  const subagentBodyMarkup = inlineResearchDelegateDetails && streamingInlineRuntime && subagents.length
    ? `
      <div class="codex-delegate-list codex-steps-list codex-subagent-list">
        ${subagents
          .map((subagent, index) => {
            const active = isStreaming && index === subagents.length - 1 && String(subagent.status || "").toLowerCase() === "running";
            return `
            <div class="codex-delegate-card codex-subagent-card${active ? " is-active" : ""}">
              <div class="codex-delegate-summary">
                <span class="codex-delegate-name">${escapeHtml(subagent.name || "subagent")}</span>
                <span class="codex-delegate-pill">${escapeHtml(renderDelegateStatus(subagent.status || ""))}</span>
              </div>
              <div class="codex-delegate-body">
                ${subagent.purpose ? `<div class="codex-delegate-line"><span class="codex-delegate-key">${escapeHtml(zhLabel("目的", "Purpose"))}</span><span class="codex-delegate-value">${escapeHtml(subagent.purpose)}</span></div>` : ""}
                ${subagent.input ? `<div class="codex-delegate-line"><span class="codex-delegate-key">${escapeHtml(zhLabel("输入", "Input"))}</span><span class="codex-delegate-value">${escapeHtml(subagent.input)}</span></div>` : ""}
                ${subagent.output ? `<div class="codex-delegate-line"><span class="codex-delegate-key">${escapeHtml(zhLabel("输出", "Output"))}</span><span class="codex-delegate-value">${escapeHtml(subagent.output)}</span></div>` : ""}
                ${
                  Array.isArray(subagent.evidence) && subagent.evidence.length
                    ? `<div class="codex-delegate-line"><span class="codex-delegate-key">${escapeHtml(currentLanguage === "zh" ? "证据" : "Evidence")}</span><span class="codex-delegate-value">${subagent.evidence.map((item) => escapeHtml(String(item || ""))).join("<br>")}</span></div>`
                    : ""
                }
              </div>
            </div>
          `;
          })
          .join("")}
      </div>
    `
    : "";

  const subagentMarkup = renderRuntimeSectionCard(
    currentLanguage === "zh" ? "子代理" : "Subagents",
    String(subagents.length || ""),
    subagentBodyMarkup,
    {
      open: subagents.some((item) => String(item?.status || "").toLowerCase() === "running"),
      tone: subagents.some((item) => String(item?.status || "").toLowerCase() === "running") ? "running" : "",
      className: "codex-runtime-section codex-runtime-subagents",
    },
  );

  const verifierBodyMarkup = inlineResearchDelegateDetails && streamingInlineRuntime && verifierReport
    ? `
      <div class="codex-delegate-card codex-verifier-card${isStreaming ? " is-active" : ""}">
        <div class="codex-delegate-summary">
          <span class="codex-delegate-name">${escapeHtml(currentLanguage === "zh" ? "验证器" : "Verifier")}</span>
          <span class="codex-delegate-pill">${escapeHtml(renderDelegateStatus(verifierReport.status || ""))}</span>
        </div>
        <div class="codex-delegate-body">
          ${verifierReport.summary ? `<div class="codex-delegate-line"><span class="codex-delegate-key">${escapeHtml(currentLanguage === "zh" ? "摘要" : "Summary")}</span><span class="codex-delegate-value">${escapeHtml(verifierReport.summary)}</span></div>` : ""}
          ${
            Array.isArray(verifierReport.checks) && verifierReport.checks.length
              ? `<div class="codex-delegate-line"><span class="codex-delegate-key">${escapeHtml(currentLanguage === "zh" ? "检查项" : "Checks")}</span><span class="codex-delegate-value">${verifierReport.checks.map((item) => `${escapeHtml(item.title || "check")} [${escapeHtml(item.status || "")}]${item.detail ? ` · ${escapeHtml(item.detail)}` : ""}`).join("<br>")}</span></div>`
              : ""
          }
          ${
            Array.isArray(verifierReport.issues) && verifierReport.issues.length
              ? `<div class="codex-delegate-line"><span class="codex-delegate-key">${escapeHtml(currentLanguage === "zh" ? "问题" : "Issues")}</span><span class="codex-delegate-value">${verifierReport.issues.map((item) => escapeHtml(String(item || ""))).join("<br>")}</span></div>`
              : ""
          }
          ${
            Array.isArray(verifierReport.next_actions) && verifierReport.next_actions.length
              ? `<div class="codex-delegate-line"><span class="codex-delegate-key">${escapeHtml(currentLanguage === "zh" ? "下一步" : "Next")}</span><span class="codex-delegate-value">${verifierReport.next_actions.map((item) => escapeHtml(String(item || ""))).join("<br>")}</span></div>`
              : ""
          }
        </div>
      </div>
    `
    : "";
  const verifierMarkup = renderRuntimeSectionCard(
    currentLanguage === "zh" ? "验证器" : "Verifier",
    verifierReport ? renderDelegateStatus(verifierReport.status || "") : "",
    verifierBodyMarkup,
    {
      open: ["running", "repair", "failed"].includes(String(verifierReport?.status || "").toLowerCase()),
      tone: ["running", "repair", "failed"].includes(String(verifierReport?.status || "").toLowerCase()) ? "running" : "",
      className: "codex-runtime-section codex-runtime-verifier",
    },
  );

  const checkpointMarkup = streamingInlineRuntime && (runtimeCheckpoints.length || branchNotes.length)
    ? `
      <div class="codex-process-list codex-steps-list codex-runtime-checkpoints">
        ${runtimeCheckpoints
          .slice(-4)
          .map((item) => `
            <div class="codex-process-step codex-process-checkpoint">
              <div class="codex-step-rail" aria-hidden="true"><span class="codex-step-dot"></span></div>
                <div class="codex-process-main">
                <div class="codex-process-label">${escapeHtml(currentLanguage === "zh" ? "检查点" : "Checkpoint")}</div>
              </div>
            </div>
          `)
          .join("")}
        ${branchNotes
          .slice(-3)
          .map((item) => `
            <div class="codex-process-step codex-process-branch">
              <div class="codex-step-rail" aria-hidden="true"><span class="codex-step-dot"></span></div>
              <div class="codex-process-main">
                <div class="codex-process-label">${escapeHtml(zhLabel("分支 / 修复", "Branch / Repair"))}</div>
                <div class="codex-process-detail">${escapeHtml(item)}</div>
              </div>
            </div>
          `)
          .join("")}
      </div>
    `
    : "";

  const runtimeSubagentMarkup = streamingInlineRuntime && subagents.length
    ? `
      <div class="research-runtime-subagents">
        ${subagents
          .map((subagent) => `
            <details class="research-runtime-card"${String(subagent.status || "").toLowerCase() === "running" ? " open" : ""}>
              <summary class="research-runtime-summary">
                <span class="research-runtime-name">${escapeHtml(subagent.name || "subagent")}</span>
                <span class="research-runtime-pill">${escapeHtml(renderDelegateStatus(subagent.status || ""))}</span>
              </summary>
              <div class="research-runtime-body">
                ${subagent.purpose ? `<div class="research-runtime-line"><span>${escapeHtml(zhLabel("目的", "Purpose"))}</span><strong>${escapeHtml(subagent.purpose)}</strong></div>` : ""}
                ${subagent.input ? `<div class="research-runtime-line"><span>${escapeHtml(zhLabel("输入", "Input"))}</span><strong>${escapeHtml(subagent.input)}</strong></div>` : ""}
                ${subagent.output ? `<div class="research-runtime-line"><span>${escapeHtml(zhLabel("输出", "Output"))}</span><strong>${escapeHtml(subagent.output)}</strong></div>` : ""}
                ${
                  Array.isArray(subagent.evidence) && subagent.evidence.length
                    ? `<div class="research-runtime-list">${subagent.evidence.map((item) => `<div class="research-review-item">${escapeHtml(String(item || ""))}</div>`).join("")}</div>`
                    : ""
                }
              </div>
            </details>
          `)
          .join("")}
      </div>
    `
    : "";

  const runtimeVerifierMarkup = streamingInlineRuntime && verifierReport
    ? `
      <details class="research-runtime-card research-runtime-verifier" open>
        <summary class="research-runtime-summary">
          <span class="research-runtime-name">${escapeHtml(zhLabel("验证器", "Verifier"))}</span>
          <span class="research-runtime-pill">${escapeHtml(renderDelegateStatus(verifierReport.status || ""))}</span>
        </summary>
        <div class="research-runtime-body">
          ${verifierReport.summary ? `<div class="research-runtime-line"><span>${escapeHtml(zhLabel("摘要", "Summary"))}</span><strong>${escapeHtml(verifierReport.summary)}</strong></div>` : ""}
          ${
            Array.isArray(verifierReport.checks) && verifierReport.checks.length
              ? `<div class="research-runtime-list">${verifierReport.checks.map((item) => `<div class="research-review-item">${escapeHtml(item.title || "check")} [${escapeHtml(item.status || "")}]${item.detail ? ` - ${escapeHtml(item.detail)}` : ""}</div>`).join("")}</div>`
              : ""
          }
          ${
            Array.isArray(verifierReport.issues) && verifierReport.issues.length
              ? `<div class="research-runtime-list">${verifierReport.issues.map((item) => `<div class="research-review-item">${escapeHtml(String(item || ""))}</div>`).join("")}</div>`
              : ""
          }
        </div>
      </details>
    `
    : "";

  const toolBodyMarkup = visibleTools.length
    ? `
      <div class="codex-tool-list codex-steps-list">
        ${visibleTools
          .map((tool, index) => {
            const argsPreview = compactToolArgs(tool.args);
            const verb = toolStatusVerb(tool.status);
            const pathLine = tool.file_path
              ? `<div class="codex-tool-path">${escapeHtml(tool.file_path)}</div>`
              : "";
            const argsLine = argsPreview
              ? `<div class="codex-tool-args">${escapeHtml(argsPreview)}</div>`
              : "";
            const resultLine =
              tool.status === "failed" && tool.result
                ? `<div class="codex-tool-result is-error">${escapeHtml(String(tool.result).slice(0, 220))}</div>`
                : "";
            const statusKey = String(tool.status || "pending");
            const active = isStreaming && index === visibleTools.length - 1 && ["pending", "approved", "executing"].includes(statusKey);
            return `
              <div class="codex-tool-card codex-tool-step codex-tool-${escapeHtml(statusKey)}${active ? " is-active" : ""}">
                <div class="codex-step-rail" aria-hidden="true">
                  <span class="codex-step-dot"></span>
                </div>
                <div class="codex-tool-main">
                  <div class="codex-tool-title-row">
                    <span class="codex-tool-title">${escapeHtml(verb)}</span>
                    <span class="codex-tool-pill">${escapeHtml(toolStatusLabel(tool.status))}</span>
                  </div>
                  <div class="codex-tool-name">${escapeHtml(tool.name || "tool")}</div>
                  ${pathLine}
                  ${argsLine}
                  ${resultLine}
                </div>
              </div>
            `;
          })
          .join("")}
      </div>
    `
    : "";
  const toolMarkup = renderRuntimeSectionCard(
    currentLanguage === "zh" ? "工具调用" : "Tool calls",
    String(visibleTools.length || ""),
    toolBodyMarkup,
    {
      open: visibleTools.some((item) => ["pending", "approved", "executing"].includes(String(item?.status || "").toLowerCase())),
      tone: visibleTools.some((item) => ["pending", "approved", "executing"].includes(String(item?.status || "").toLowerCase())) ? "running" : "",
      className: "codex-runtime-section codex-runtime-tools",
    },
  );

  const permissionMarkup = streamingInlineRuntime && permission
    ? `
      <div class="codex-approval-card codex-tool-step codex-approval-step">
        <div class="codex-step-rail" aria-hidden="true">
          <span class="codex-step-dot"></span>
        </div>
        <div class="codex-approval-copy">
          <div class="codex-approval-title">${escapeHtml(currentLanguage === "zh" ? "等待工具授权" : "Awaiting tool approval")}</div>
          <div class="codex-approval-meta">${escapeHtml(permission.name || "")}${permission.risk ? ` · ${escapeHtml(permission.risk || "")}` : ""}</div>
        </div>
        <div class="codex-approval-actions">
          <button class="codex-approval-button" type="button" data-permission-action="deny">${escapeHtml(currentLanguage === "zh" ? "拒绝" : "Deny")}</button>
          <button class="codex-approval-button is-primary" type="button" data-permission-action="approve">${escapeHtml(currentLanguage === "zh" ? "批准" : "Approve")}</button>
        </div>
      </div>
    `
    : "";

  const diffMarkup = !isStreaming && visibleDiffs.length
    ? renderAssistantDiffMarkup(visibleDiffs, { inline: true })
    : "";
  const operationDetailPanels = buildOperationDetailPanels(turn, { isStreaming });
  const operationBodyMarkup = [
    permissionMarkup,
    (operationDetailPanels || checkpointMarkup || diffMarkup)
      ? `<div class="codex-operation-detail-stack">${operationDetailPanels}${checkpointMarkup}${diffMarkup}</div>`
      : "",
  ].filter(Boolean).join("");

  const thinkingMarkup = isStreaming
    ? ""
    : (
      thinking.length
        ? thinking
            .map(
              (block, index) => `
                <details class="thinking-block codex-thinking-block">
                  <summary>${renderThinkingSummaryLabel(index, false)}</summary>
                  <div class="thinking-content markdown-body">${renderMarkdown(block.content || "")}</div>
                </details>
              `,
            )
            .join("")
        : ""
    );

  const streamingParts = isStreaming
    ? renderStreamingAssistantContent(visibleText, {
        placeholder: pendingAssistantPlaceholderText(),
        streaming: true,
      })
    : null;

  const worklogStatusesHtml = "";

  const textMarkup = isStreaming
    ? `
      <div class="codex-answer codex-answer-streaming${streamingParts?.isEmpty ? " codex-answer-empty" : ""}">
        ${renderTurnStoryline(turn, {
          streaming: true,
          streamingParts,
          fallbackText: visibleText,
          overrideText: shouldSuppressInlineCode ? visibleText : "",
        })}
        ${worklogStatusesHtml ? `<div class="codex-runtime-statuses" data-runtime-statuses>${worklogStatusesHtml}</div>` : ""}
      </div>
    `
    : (turn?.textSegments?.length || turn?.streamMoments?.length)
      ? `<div class="codex-answer">${renderTurnStoryline(turn, { fallbackText: visibleText, overrideText: shouldSuppressInlineCode ? visibleText : "" })}</div>${worklogStatusesHtml ? `<div class="codex-runtime-statuses">${worklogStatusesHtml}</div>` : ""}`
      : visibleText
        ? `<div class="codex-answer markdown-body">${renderMarkdown(visibleText)}</div>${worklogStatusesHtml ? `<div class="codex-runtime-statuses">${worklogStatusesHtml}</div>` : ""}`
        : worklogStatusesHtml ? `<div class="codex-runtime-statuses">${worklogStatusesHtml}</div>` : "";
  const decisionCardMarkup = !isStreaming ? renderAssistantDecisionCard(decisionPresentation.card) : "";

  const runtimePanelContent = [
    operationBodyMarkup,
  ].filter(Boolean).join("");
  const shouldShowRuntimePanel = runtimePanelContent && autoOpenActivityPanel;
  const runtimePanelMarkup = shouldShowRuntimePanel
    ? renderAssistantRuntimePanel(runtimePanelContent, {
        title: currentLanguage === "zh" ? "操作" : "Activity",
        meta: runtimeSummaryParts.join(" / "),
        open: isStreaming && autoOpenActivityPanel,
        tone: isStreaming ? "running" : "",
      })
    : "";
  return `
    <div class="codex-stream-phase"${isStreaming ? streamAnimationStyle(turn) : ""}>
      <article class="message-row assistant-row assistant-message-row codex-turn-row">
        <div class="codex-turn-shell">
          ${runtimeHead}
          <div data-thinking-host>${showStreamingThinking ? renderStreamingThinkingIndicator() : ""}</div>
          ${thinkingMarkup}
          ${textMarkup}
          ${runtimePanelMarkup}
          ${decisionCardMarkup}
        </div>
      </article>
    </div>
  `;
}

function appendUserBubble(content) {
  if (!messageStream) return null;
  messageStream.classList.remove("is-empty");
  messageStream.querySelector(".empty-state")?.remove();
  const row = document.createElement("article");
  row.className = "message-row user-row user-message-row";
  row.innerHTML = `
    <div class="message-bubble user-bubble">
      <div class="message-body markdown-body">${renderMarkdown(content || "")}</div>
    </div>
  `;
  messageStream.appendChild(row);
  scrollMessageStreamToBottom(true);
  return row;
}

function appendThinkingContent(delta) {
  if (!activeAssistantTurn) return;
  activeAssistantTurn.isThinkingPhase = true;
  const thinking = Array.isArray(activeAssistantTurn.thinking) ? activeAssistantTurn.thinking : [];
  let currentBlock;
  if (thinking.length === 0) {
    currentBlock = { content: "", collapsed: false };
    thinking.push(currentBlock);
  } else {
    currentBlock = thinking[thinking.length - 1];
  }
  currentBlock.content = (currentBlock.content || "") + delta;
  activeAssistantTurn.thinking = thinking;
  preservedThinking = thinking.map((item) => ({ ...item }));
  pendingAssistantThinkingDirty = true;
}

function appendAssistantBubble(content) {
  if (!messageStream) return null;
  messageStream.classList.remove("is-empty");
  messageStream.querySelector(".empty-state")?.remove();
  const row = document.createElement("article");
  row.className = "codex-turn-anchor";
  if (!activeAssistantTurn) resetActiveAssistantTurn();
  if (content) {
    activeAssistantTurn.text = content || "";
  }
  messageStream.appendChild(row);
  pendingAssistantBubble = row;
  pendingAssistantBubble.innerHTML = renderPendingAssistantShell();
  bindTurnInteractionHandlers(pendingAssistantBubble);
  bindPendingAssistantNodes(pendingAssistantBubble);
  refreshPendingAssistantBubble();
  scrollMessageStreamToBottom(true);
  return row;
}

function updateAssistantBubble(content) {
  if (!pendingAssistantBubble) {
    pendingAssistantBubble = appendAssistantBubble(content);
  }
  if (!activeAssistantTurn) resetActiveAssistantTurn();
  const keepBottom = isNearMessageStreamBottom();
  const deltaContent = sanitizeMessageContent(String(content || ""));
  activeAssistantTurn.receivedDelta = true;
  if (deltaContent.trim()) {
    activeAssistantTurn.isThinkingPhase = false;
  }
  if (deltaContent.trim()) {
    activeAssistantTurn.runtimeNarration = "";
  }
  const derivedMoment = corroborateAssistantOperationalMoment(
    extractAssistantOperationalMoment(deltaContent),
    activeAssistantTurn,
  );
  const suppressInlineDump = !derivedMoment && shouldSuppressInlineAssistantCode(deltaContent, activeAssistantTurn?.diffs || []);
  let createdNewSegment = false;
  if (derivedMoment) {
    pushAssistantStreamMoment(derivedMoment);
  } else if (suppressInlineDump) {
    activeAssistantTurn.streamingAnchorText = "";
    activeAssistantTurn.suppressedInlineContent = true;
    pendingAssistantStoryDirty = true;
  } else {
    activeAssistantTurn.streamingAnchorText = mergeStreamingTextDelta(
      String(activeAssistantTurn.streamingAnchorText || ""),
      deltaContent,
    );
    createdNewSegment = pushTurnTextSegment(activeAssistantTurn, deltaContent, {
      forceNew: activeAssistantTurn.lastStreamEventKind !== "text",
    });
    if (createdNewSegment) {
      pendingAssistantStoryDirty = true;
    }
  }
  captureAssistantOperationNarration(deltaContent);
  if (activeAssistantTurn && assistantTextLooksLikeProcessNarration(deltaContent) && !derivedMoment) {
    pushAssistantWorklog({
      kind: "progress",
      text: deltaContent,
      dedupeKey: `delta-progress:${normalizeText(deltaContent)}`,
    });
  }
  ensurePendingAssistantBubbleForRuntime();
  if (createdNewSegment) {
    refreshPendingAssistantBubble();
    if (keepBottom) {
      scrollMessageStreamToBottom(true);
    }
    return;
  }
  schedulePendingAssistantTextSync({ keepBottom });
}

function updateRuntimeNarration(text) {
  if (!activeAssistantTurn) return;
  const next = normalizeAgentStageNarration(text);
  if (!next) return;
  const current = cleanDisplayText(String(activeAssistantTurn.runtimeNarration || "").trim(), "");
  activeAssistantTurn.runtimeNarration = current
    ? combineAssistantSegments(current, next)
    : next;
}

function createThinkingBlock(message) {
  const row = document.createElement("article");
  row.className = "message-row assistant-row assistant-message-row";
  const details = document.createElement("details");
  details.className = "thinking-block codex-thinking-block";
  details.open = false;
  details.innerHTML = `
    <summary>${renderThinkingSummaryLabel(0, false)}</summary>
    <div class="thinking-content markdown-body">${renderMarkdown(message.content || "")}</div>
  `;
  row.appendChild(details);
  return row;
}

function finalizeVisibleAssistantBubble(messages, runtimeTurn = null) {
  if (!pendingAssistantBubble) return false;
  const turns = groupMessagesIntoTurns(visibleConversationMessages(messages || []));
  const finalAssistantTurn = [...turns].reverse().find((turn) => turn?.kind === "assistant_turn" && turn?.data);
  if (!finalAssistantTurn?.data) return false;
  const finalData = runtimeTurn
      ? (() => {
        const persistedText = cleanDisplayText(completionFallbackAssistantContent(finalAssistantTurn.data), "");
        const runtimeText = cleanDisplayText(completionFallbackAssistantContent(runtimeTurn), "");
        const merged = mergeAssistantTurnData(finalAssistantTurn.data, runtimeTurn, {
          preferLiveText: !persistedText,
        });
        const mergedRenderableText = cleanDisplayText(completionFallbackAssistantContent(merged), "");
        const selectedText = String(persistedText || mergedRenderableText || runtimeText || "");
        // Force thinking from global cache — the single source of truth
        if (preservedThinking.length) {
          merged.thinking = preservedThinking.map((item) => ({ ...item }));
        } else {
          const liveThinking = Array.isArray(runtimeTurn?.thinking) ? runtimeTurn.thinking : [];
          const persistedThinking = Array.isArray(finalAssistantTurn?.data?.thinking)
            ? finalAssistantTurn.data.thinking : [];
          merged.thinking = liveThinking.length >= persistedThinking.length
            ? liveThinking.map((item) => ({ ...item }))
            : persistedThinking.map((item) => ({ ...item }));
        }
        return {
          ...merged,
          text: selectedText,
          receivedDelta: false,
        };
      })()
    : finalAssistantTurn.data;
  const keepBottom = isNearMessageStreamBottom();
  pendingAssistantBubble.innerHTML = renderAssistantTurn(finalData, { streaming: false });
  bindTurnInteractionHandlers(pendingAssistantBubble);
  pendingAssistantBubble.querySelectorAll(".codex-runtime-panel-shell[open]").forEach((details) => {
    animateDetailsToggle(details, false);
  });
  pendingAssistantBubble.querySelectorAll(".codex-thinking-block").forEach((details) => {
    const shell = details;
    if (shell instanceof HTMLDetailsElement) {
      shell.open = false;
    }
  });
  resetPendingAssistantRenderState();
  pendingAssistantBubble = null;
  pendingUserBubble = null;
  lastVisibleCompletionSignature = visibleMessagesSignature(visibleConversationMessages(messages || []));
  if (keepBottom) {
    scrollMessageStreamToBottom(true);
  }
  return true;
}

function ensureVisibleAssistantCompletionMessage(messages, fallbackTurn = activeAssistantTurn) {
  const nextMessages = Array.isArray(messages) ? messages.slice() : [];
  const fallbackText = completionFallbackAssistantContent(fallbackTurn);
  const fallbackChoices = fallbackTurn?.assistantChoices
    && Array.isArray(fallbackTurn.assistantChoices.options)
    && fallbackTurn.assistantChoices.options.length
      ? {
          title: cleanDisplayText(fallbackTurn.assistantChoices.title || "", zhLabel("选择下一步", "Choose next step")),
          options: fallbackTurn.assistantChoices.options
            .map((item) => cleanDisplayText(item || "", ""))
            .filter(Boolean),
        }
      : null;
  if (!fallbackText && !fallbackChoices) return nextMessages;
  const fallbackCore = cleanDisplayText(assistantPrimaryReplyCore(fallbackText), "");

  const turns = groupMessagesIntoTurns(visibleConversationMessages(nextMessages));
  const lastAssistantTurn = [...turns].reverse().find((turn) => turn?.kind === "assistant_turn" && turn?.data);
  const lastAssistantText = cleanDisplayText(completionFallbackAssistantContent(lastAssistantTurn?.data), "");
  const lastAssistantCore = cleanDisplayText(assistantPrimaryReplyCore(lastAssistantText), "");
  const lastAssistantHasChoices = Boolean(
    lastAssistantTurn?.data?.assistantChoices
    && Array.isArray(lastAssistantTurn.data.assistantChoices.options)
    && lastAssistantTurn.data.assistantChoices.options.length,
  );
  if (
    (lastAssistantHasChoices && fallbackChoices)
    || (lastAssistantText &&
    (lastAssistantText === fallbackText
      || lastAssistantText.includes(fallbackText)
      || fallbackText.includes(lastAssistantText)
      || (fallbackCore && lastAssistantCore && (lastAssistantCore === fallbackCore || lastAssistantCore.includes(fallbackCore) || fallbackCore.includes(lastAssistantCore)))))
  ) {
    return nextMessages;
  }

  const fallbackMessage = {
    kind: "message",
    role: "assistant",
    content: fallbackText || "",
  };
  if (fallbackChoices) {
    fallbackMessage.assistant_choices = fallbackChoices;
  }
  nextMessages.push(fallbackMessage);
  return nextMessages;
}

function createMessageRow(message) {
  if (message.kind === "thinking") {
    return createThinkingBlock(message);
  }
  if (message.kind === "tool" || message.kind === "tool_result" || message.kind === "diff") {
    return null;
  }

  const row = document.createElement("article");
  const role = String(message.role || "assistant");
  const cleanedMessageContent = displayMarkdownText(message.content || "");

  if (role === "user") {
    row.className = "message-row user-row user-message-row";
    row.innerHTML = `
      <div class="message-bubble user-bubble">
        <div class="message-body markdown-body">${renderMarkdown(cleanedMessageContent)}</div>
      </div>
    `;
    return row;
  }

  row.className = `message-row ${role}-row assistant-message-row`;
  row.innerHTML = `
    <div class="assistant-card">
      <div class="message-body markdown-body">${renderMarkdown(cleanedMessageContent)}</div>
    </div>
  `;
  return row;
}

function groupMessagesIntoTurns(messages) {
  const turns = [];
  let currentAssistant = null;
  let lastStructuredToolResultContent = "";

  const flushAssistant = () => {
    if (currentAssistant) {
      turns.push({ kind: "assistant_turn", data: currentAssistant });
      currentAssistant = null;
    }
  };

  (messages || []).forEach((message) => {
    if (!message) return;
    if (message.kind === "message" && message.role === "user") {
      flushAssistant();
      turns.push({
        kind: "user",
        data: {
          ...message,
          content: displayMarkdownText(message.content || ""),
        },
      });
      return;
    }

    if (!currentAssistant) {
      currentAssistant = createEmptyAssistantTurn();
    }

    if (message.kind === "message" && (message.role === "assistant" || message.role === "system" || message.role === "error")) {
      if (message.assistant_choices && Array.isArray(message.assistant_choices.options) && message.assistant_choices.options.length) {
        currentAssistant.assistantChoices = {
          title: cleanDisplayText(message.assistant_choices.title || "", zhLabel("选择下一步", "Choose next step")),
          options: message.assistant_choices.options.map((item) => cleanDisplayText(item || "", "")).filter(Boolean),
        };
      }
      const nextContent = displayMarkdownText(message.content || "");
      const nextRenderableText = sanitizeMessageContent(String(nextContent || "")).trim();
      const operationalMoment = extractAssistantOperationalMoment(nextRenderableText);
      const shouldKeepAsVisibleText = Boolean(
        nextRenderableText
        && !looksLikeOperationalContentDump(nextRenderableText),
      ) && !operationalMoment;
      if (operationalMoment) {
        pushTurnStreamMoment(currentAssistant, {
          ...operationalMoment,
          timestamp: Date.now(),
        });
      }
      const nextLooksOperational = !isAssistantPrimaryReplyText(nextContent)
        && !isAssistantFailureSummaryText(nextContent)
        && !isAssistantVerificationAppendixText(nextContent);
      if (nextLooksOperational) {
        const currentNarration = cleanDisplayText(String(currentAssistant.progressNarration || "").trim(), "");
        currentAssistant.progressNarration = currentNarration
          ? mergeAssistantText(currentNarration, nextContent)
          : nextContent;
        pushTurnWorklogEntry(currentAssistant, {
          kind: assistantTextLooksLikeProcessNarration(nextContent) ? "progress" : "activity",
          text: nextContent,
          dedupeKey: `message-progress:${normalizeText(nextContent)}`,
        });
      }
      const hasPrimaryAssistantText = isAssistantPrimaryReplyText(currentAssistant.text || "");
      const nextLooksAncillary = isAssistantFailureSummaryText(nextContent) || isAssistantVerificationAppendixText(nextContent);
      if (hasPrimaryAssistantText && nextLooksAncillary) {
        lastStructuredToolResultContent = "";
        return;
      }
      if (lastStructuredToolResultContent) {
        const normalizedNext = normalizedAssistantSubstantiveContent(nextContent);
        const shouldReplaceWithToolResult =
          !normalizedNext
          || assistantTextLooksLikeProcessNarration(nextContent)
          || isAssistantFailureSummaryText(nextContent);
        if (shouldReplaceWithToolResult) {
          currentAssistant.text = preferAssistantMessageContent(
            currentAssistant.text || "",
            lastStructuredToolResultContent,
          );
          replaceTurnTextSegments(currentAssistant, currentAssistant.text, {
            timestamp: Date.now(),
          });
          lastStructuredToolResultContent = "";
          return;
        }
      }
      currentAssistant.text = preferAssistantMessageContent(
        currentAssistant.text || "",
        nextContent,
      );
      if (shouldKeepAsVisibleText) {
        pushTurnTextSegment(currentAssistant, nextContent, {
          forceNew: currentAssistant.lastStreamEventKind !== "text",
          timestamp: Date.now(),
        });
      }
      lastStructuredToolResultContent = "";
      return;
    }

    if (message.kind === "thinking") {
      currentAssistant.thinking.push({
        content: message.content || "",
        collapsed: Boolean(message.collapsed),
      });
      return;
    }

    if (message.kind === "tool") {
      const callId = String(message.call_id || "").trim()
        || `legacy-tool:${currentAssistant.tools.length}:${message.tool_name || "tool"}:${message.file_path || ""}`;
      const existingTool = currentAssistant.tools.find((item) => item.call_id === callId);
      if (existingTool) {
        existingTool.name = message.tool_name || existingTool.name || "tool";
        existingTool.status = message.status || existingTool.status || "pending";
        existingTool.args = message.tool_args || existingTool.args || null;
        existingTool.file_path = message.file_path || existingTool.file_path || "";
      } else {
        currentAssistant.tools.push({
          call_id: callId,
          name: message.tool_name || "tool",
          status: message.status || "pending",
          args: message.tool_args || null,
          file_path: message.file_path || "",
          risk: "",
          result: "",
          success: null,
        });
      }
      pushTurnStreamMoment(currentAssistant, describeToolMoment({
        call_id: callId,
        name: message.tool_name || "tool",
        status: message.status || "pending",
        file_path: message.file_path || "",
        params: message.tool_args || null,
      }));
      return;
    }

    if (message.kind === "tool_result") {
      const resultCallId = String(message.call_id || "").trim();
      const tool = currentAssistant.tools.find((item) => item.call_id === resultCallId)
        || (!resultCallId
          ? [...currentAssistant.tools].reverse().find((item) => ["pending", "approved", "executing", "running"].includes(String(item.status || "").toLowerCase()))
          : null);
      if (tool) {
        tool.result = message.content || "";
        tool.success = message.success ?? null;
        tool.status = message.status || tool.status || "complete";
        pushTurnStreamMoment(currentAssistant, describeToolMoment(tool));
      }
      const structuredToolResult = extractStructuredToolResultContent(message.content || "");
      if (structuredToolResult) {
        lastStructuredToolResultContent = structuredToolResult;
      }
      return;
    }

    if (message.kind === "diff") {
      const diffPath = message.file_path || "";
      if (!diffPath) return;
      const existingDiff = currentAssistant.diffs.find((item) => item.path === diffPath);
      if (existingDiff) {
        existingDiff.added = message.added || 0;
        existingDiff.removed = message.removed || 0;
        existingDiff.before_content = String(message.before_content || existingDiff.before_content || "");
      } else {
        currentAssistant.diffs.push({
          path: diffPath,
          added: message.added || 0,
          removed: message.removed || 0,
          before_content: String(message.before_content || ""),
        });
      }
      pushTurnStreamMoment(currentAssistant, describeEditedFileMoment({
        path: diffPath,
        added: message.added || 0,
        removed: message.removed || 0,
      }));
      return;
    }

    if (message.kind === "subagent" && message.subagent) {
      const id = String(message.subagent?.id || message.subagent?.name || "").trim() || `${Date.now()}`;
      const existing = currentAssistant.subagents.find((item) => String(item.id || "") === id);
      if (existing) {
        Object.assign(existing, { ...message.subagent, id });
      } else {
        currentAssistant.subagents.push({ ...message.subagent, id });
      }
      return;
    }

    if (message.kind === "verification" && message.verifier) {
      currentAssistant.verifierReport = { ...message.verifier };
    }
  });

  flushAssistant();
  return turns;
}

function mergeActiveRuntimeIntoTurns(turns) {
  if (
    !Array.isArray(turns)
    || !activeAssistantTurn
    || !isVisibleSessionRunning()
    || pendingAssistantBubble
  ) return turns;
  const lastAssistantIndex = turns.map((item) => item?.kind).lastIndexOf("assistant_turn");
  if (lastAssistantIndex < 0) return turns;
  const entry = turns[lastAssistantIndex];
  if (!entry?.data) return turns;
  const merged = mergeAssistantTurnData(entry.data, activeAssistantTurn, { preferLiveText: true });
  const nextTurns = turns.slice();
  nextTurns[lastAssistantIndex] = { ...entry, data: merged };
  return nextTurns;
}

function bindTurnInteractionHandlers(scope = document) {
  scope.querySelectorAll("[data-runtime-toggle]").forEach((summary) => {
    if (!(summary instanceof HTMLElement)) return;
    if (summary.dataset.boundRuntimeToggle === "true") return;
    summary.dataset.boundRuntimeToggle = "true";
    summary.addEventListener("click", (event) => {
      const details = summary.closest("details[data-runtime-panel]");
      if (!(details instanceof HTMLDetailsElement)) return;
      event.preventDefault();
      if (details.dataset.animating === "true") return;
      animateDetailsToggle(details, !details.open);
    });
  });

  scope.querySelectorAll("[data-open-workspace-file]").forEach((button) => {
    if (button.dataset.boundWorkspaceOpen === "true") return;
    button.dataset.boundWorkspaceOpen = "true";
    button.addEventListener("click", async () => {
      const path = button.getAttribute("data-open-workspace-file") || "";
      const lineNumber = Number(button.getAttribute("data-open-workspace-line") || 0) || null;
      const column = Number(button.getAttribute("data-open-workspace-column") || 0) || null;
      if (!path) return;
      try {
        await openWorkspaceFileAt(path, lineNumber, column);
      } catch (error) {
        console.error(error);
      showToast(appErrorMessage(error, "workspace", "toastSendFailed"));
      }
    });
  });

  scope.querySelectorAll("[data-review-path]").forEach((button) => {
    if (button.dataset.boundReviewPath === "true") return;
    button.dataset.boundReviewPath = "true";
    button.addEventListener("click", async () => {
      const path = button.getAttribute("data-review-path") || "";
      if (!path) return;
      activeReviewFilePath = activeReviewFilePath === path ? null : path;
      try {
        if (activeReviewFilePath) {
          await ensureReviewDetail(path);
        }
        renderMessages(bootstrapData?.messages || [], { preserveScroll: true });
      } catch (error) {
        console.error(error);
        showToast(t("reviewError"));
      }
    });
  });

  scope.querySelectorAll("[data-undo-key]").forEach((button) => {
    if (button.dataset.boundUndoKey === "true") return;
    button.dataset.boundUndoKey = "true";
    button.addEventListener("click", async () => {
      const key = button.getAttribute("data-undo-key") || "";
      const snapshot = undoSnapshotCache.get(key);
      if (!snapshot?.path) return;
      try {
        const response = await hostClient.workspace.undoFile(snapshot.path, snapshot.beforeContent || "");
        if (!response.ok) {
          const errorText = await response.text();
          throw new Error(errorText || `undo failed: ${response.status}`);
        }
        const payload = await response.json();
        const restoredFile = payload?.data?.file || null;
        if (restoredFile && String(currentWorkspaceFile?.path || "") === String(restoredFile.path || "")) {
          renderWorkspaceFile(restoredFile);
          renderWorkspaceTabs();
        }
        setDiffReviewStatus({ path: snapshot.path }, "reverted");
        if (activeReviewFilePath === snapshot.path) {
          activeReviewFilePath = null;
        }
        await loadBootstrap();
        showToast(currentLanguage === "zh" ? "已撤销本次编辑" : "Edit undone");
      } catch (error) {
        console.error(error);
        showToast(error?.message || t("toastSendFailed"));
      }
    });
  });

  scope.querySelectorAll("[data-accept-path]").forEach((button) => {
    if (button.dataset.boundAcceptPath === "true") return;
    button.dataset.boundAcceptPath = "true";
    button.addEventListener("click", () => {
      const path = button.getAttribute("data-accept-path") || "";
      if (!path) return;
      setDiffReviewStatus({ path }, "accepted");
      renderMessages(bootstrapData?.messages || [], { preserveScroll: true });
    });
  });

  scope.querySelectorAll("[data-permission-action]").forEach((button) => {
    if (button.dataset.boundPermissionAction === "true") return;
    button.dataset.boundPermissionAction = "true";
    button.addEventListener("click", async () => {
      const action = button.getAttribute("data-permission-action") || "";
      if (!pendingPermissionRequest || !currentStreamingSessionId) return;
      try {
        const response = action === "approve"
          ? await hostClient.toolApproval.approve(
              currentStreamingSessionId,
              pendingPermissionRequest.call_id,
            )
          : await hostClient.toolApproval.deny(
              currentStreamingSessionId,
              pendingPermissionRequest.call_id,
            );
        if (!response.ok) {
          const errorText = await response.text();
          throw new Error(errorText || `permission failed: ${response.status}`);
        }
        pendingPermissionRequest = null;
        if (activeAssistantTurn) {
          activeAssistantTurn.permission = null;
        }
        schedulePendingAssistantStatusSync();
        renderPermissionStrip();
      } catch (error) {
        console.error(error);
        showToast(error?.message || t("toastSendFailed"));
      }
    });
  });

  scope.querySelectorAll("[data-paper-workspace-section]").forEach((button) => {
    if (button.dataset.boundPaperWorkspaceSection === "true") return;
    button.dataset.boundPaperWorkspaceSection = "true";
    button.addEventListener("click", () => {
      paperWorkspaceState.sectionId = cleanDisplayText(button.getAttribute("data-paper-workspace-section") || "", "");
      paperWorkspaceState.label = cleanDisplayText(button.getAttribute("data-paper-workspace-title") || "", "");
      renderResearch(bootstrapData?.research || null);
    });
  });

  scope.querySelectorAll("[data-decision-option]").forEach((button) => {
    if (!(button instanceof HTMLElement)) return;
    if (button.dataset.boundDecisionOption === "true") return;
    button.dataset.boundDecisionOption = "true";
    button.addEventListener("click", async () => {
      const value = button.getAttribute("data-decision-option") || "";
      if (!value || !messageInput) return;
      if (isSending) return;
      messageInput.value = value;
      await sendMessage();
    });
  });

  scope.querySelectorAll("[data-decision-custom-submit]").forEach((button) => {
    if (!(button instanceof HTMLElement)) return;
    if (button.dataset.boundDecisionSubmit === "true") return;
    button.dataset.boundDecisionSubmit = "true";
    button.addEventListener("click", async () => {
      const card = button.closest(".codex-decision-card");
      const input = card?.querySelector("[data-decision-custom-input]");
      if (!(input instanceof HTMLTextAreaElement) || !messageInput) return;
      const value = String(input.value || "").trim();
      if (!value) return;
      messageInput.value = value;
      await sendMessage();
    });
  });
}

function animateDetailsToggle(details, open) {
  if (!(details instanceof HTMLDetailsElement)) return;
  const content = details.querySelector(".codex-runtime-panel-body, .codex-runtime-card-body");
  if (!(content instanceof HTMLElement)) {
    details.open = open;
    return;
  }
  const startHeight = details.offsetHeight;
  details.open = true;
  const expandedHeight = details.offsetHeight;
  const summary = details.querySelector("summary");
  if (summary instanceof HTMLElement) {
    summary.style.pointerEvents = "none";
  }
  if (!open) {
    details.open = true;
  }
  const endHeight = open ? expandedHeight : (summary instanceof HTMLElement ? summary.offsetHeight : Math.min(startHeight, 44));
  details.style.height = `${startHeight}px`;
  details.style.overflow = "clip";
  details.dataset.animating = "true";
  requestAnimationFrame(() => {
    details.style.transition = "height 220ms cubic-bezier(0.2, 0.82, 0.2, 1), opacity 220ms cubic-bezier(0.2, 0.82, 0.2, 1)";
    details.style.height = `${endHeight}px`;
    details.style.opacity = open ? "1" : "0.98";
  });
  window.setTimeout(() => {
    details.open = open;
    details.style.removeProperty("height");
    details.style.removeProperty("overflow");
    details.style.removeProperty("transition");
    details.style.removeProperty("opacity");
    delete details.dataset.animating;
    if (summary instanceof HTMLElement) {
      summary.style.removeProperty("pointer-events");
    }
  }, 240);
}

function renderEmptyState() {
  const researchText = currentLanguage === "zh" ? "今天想探索什么？" : "What would you like to explore today?";
  const chatText = currentLanguage === "zh" ? "告诉我你在想什么" : "Tell me what you're thinking";
  return `
    <div class="empty-state">
      <div class="empty-state-perspective" aria-hidden="true">
        <div class="empty-state-cube">
          <div class="empty-state-face empty-state-face-research">${escapeHtml(researchText)}</div>
          <div class="empty-state-face empty-state-face-chat">${escapeHtml(chatText)}</div>
        </div>
      </div>
    </div>
  `;
}

function shouldShowAgentPreludeBackground(messages = []) {
  const visibleMessages = (messages || []).filter(
    (message) =>
      message &&
      (message.kind === "message" ||
        message.kind === "thinking" ||
        message.kind === "tool" ||
        message.kind === "tool_result" ||
        message.kind === "diff"),
  );
  return currentWorkspaceMode === "research" && !visibleMessages.length && !hasResearchStartedForCurrentSession();
}

async function ensureAgentPreludeBackground() {
  if (!agentPreludeSplineFrame) return;
  const src = agentPreludeSplineFrame.getAttribute("src") || "";
  if (!src || agentPreludeSplineFrame.dataset.loaded === "true") return;
  agentPreludeSplineFrame.dataset.loaded = "true";
}

function syncAgentPreludeBackground(messages = []) {
  if (!agentPreludeBackground) return;
  const visible = shouldShowAgentPreludeBackground(messages);
  window.clearTimeout(agentPreludeHideTimer);
  if (visible) {
    agentPreludeBackground.hidden = false;
    agentPreludeBackground.classList.add("is-visible");
    ensureAgentPreludeBackground().catch(() => {});
  } else {
    agentPreludeBackground.classList.remove("is-visible");
    agentPreludeHideTimer = window.setTimeout(() => {
      if (!agentPreludeBackground.classList.contains("is-visible")) {
        agentPreludeBackground.hidden = true;
      }
    }, 560);
  }
}

function renderMessages(messages, options = {}) {
  if (!messageStream) return;
  const preserveScroll = Boolean(options.preserveScroll);
  if (preserveScroll) {
    captureMessageScrollPosition();
  }
  if (shouldPreserveStreamingConversationDom()) {
    syncAgentPreludeBackground(messages || []);
    if (preserveScroll) {
      requestAnimationFrame(() => restoreMessageScrollPosition());
    }
    return;
  }
  const visibleMessages = (messages || []).filter(
    (message) =>
      message &&
      (message.kind === "message" ||
        message.kind === "thinking" ||
        message.kind === "tool" ||
        message.kind === "tool_result" ||
        message.kind === "diff" ||
        message.kind === "subagent" ||
        message.kind === "verification"),
  );

  syncAgentPreludeBackground(visibleMessages);
  messageStream.innerHTML = "";
  messageStream.classList.toggle("is-empty", !visibleMessages.length);
  pendingUserBubble = null;
  pendingAssistantBubble = null;

  if (!visibleMessages.length) {
    messageStream.innerHTML = renderEmptyState();
    resetPendingAssistantRenderState();
    return;
  }

  const turns = mergeActiveRuntimeIntoTurns(groupMessagesIntoTurns(visibleMessages));
  // Inject preservedThinking into the last assistant turn so it survives any re-render
  if (preservedThinking.length) {
    const lastAssistantIdx = turns.map((t) => t?.kind).lastIndexOf("assistant_turn");
    if (lastAssistantIdx >= 0 && turns[lastAssistantIdx]?.data) {
      turns[lastAssistantIdx].data = {
        ...turns[lastAssistantIdx].data,
        thinking: preservedThinking.map((item) => ({ ...item })),
      };
    }
  }
  turns.forEach((turn) => {
    if (turn.kind === "user") {
      const row = createMessageRow(turn.data);
      if (row) messageStream.appendChild(row);
      return;
    }
    if (turn.kind === "assistant_turn") {
      const wrapper = document.createElement("div");
      wrapper.className = "codex-turn-anchor";
      wrapper.innerHTML = renderAssistantTurn(turn.data, { streaming: false });
      wrapper.querySelectorAll(".codex-runtime-panel-shell[open]").forEach((details) => {
        const shell = details;
        if (shell instanceof HTMLDetailsElement) {
          shell.open = false;
        }
      });
      messageStream.appendChild(wrapper);
    }
  });
  const conversationPaperPrompt = renderConversationPaperWorkflowPrompt(bootstrapData?.research || null);
  if (conversationPaperPrompt) {
    const wrapper = document.createElement("div");
    wrapper.className = "codex-turn-anchor conversation-paper-entry-anchor";
    wrapper.innerHTML = conversationPaperPrompt;
    messageStream.appendChild(wrapper);
  }
  const conversationPdfEntry = renderConversationPaperPdfEntry(bootstrapData?.research || null);
  if (conversationPdfEntry) {
    const wrapper = document.createElement("div");
    wrapper.className = "codex-turn-anchor conversation-paper-entry-anchor";
    wrapper.innerHTML = conversationPdfEntry;
    messageStream.appendChild(wrapper);
  }
  bindTurnInteractionHandlers(messageStream);
  messageStream.querySelectorAll("[data-inline-link]").forEach((link) => {
    link.addEventListener("click", (event) => {
      const href = cleanDisplayText(link.getAttribute("data-inline-link") || "");
      if (!href) return;
      event.preventDefault();
      openUrlInAppBrowser(href).catch((error) => {
        console.error(error);
        showToast(cleanDisplayText(error?.message || "") || t("toastSendFailed"));
      });
    });
  });
  resetPendingAssistantRenderState();
  requestAnimationFrame(() => {
    if (preserveScroll) {
      restoreMessageScrollPosition();
    } else if (messageStream) {
      scrollMessageStreamToBottom(true);
    }
  });
}

function workspaceFlattenFiles(entries, bucket = []) {
  (entries || []).forEach((entry) => {
    if (entry?.kind === "file") {
      bucket.push(entry);
    }
    if (Array.isArray(entry?.children) && entry.children.length) {
      workspaceFlattenFiles(entry.children, bucket);
    }
  });
  return bucket;
}

function collectWorkspaceDirs(entries, bucket = []) {
  (entries || []).forEach((entry) => {
    if (entry?.kind === "directory") {
      bucket.push(entry.path);
      if (Array.isArray(entry.children) && entry.children.length) {
        collectWorkspaceDirs(entry.children, bucket);
      }
    }
  });
  return bucket;
}

function expandWorkspacePathAncestors(path) {
  const normalized = String(path || "").replace(/\\/g, "/");
  if (!normalized) return;
  const parts = normalized.split("/").filter(Boolean);
  let current = "";
  for (let index = 0; index < parts.length - 1; index += 1) {
    current = current ? `${current}/${parts[index]}` : parts[index];
    expandedWorkspaceDirs.add(current);
  }
}

function workspaceFileExtension(path) {
  const name = String(path || "").split("/").pop() || "";
  const index = name.lastIndexOf(".");
  if (index <= 0) return "";
  return name.slice(index + 1).toLowerCase();
}

function workspaceIconType(path) {
  const extension = workspaceFileExtension(path);
  return (
    {
      rs: "rust",
      js: "javascript",
      jsx: "javascript",
      mjs: "javascript",
      cjs: "javascript",
      ts: "typescript",
      tsx: "typescript",
      py: "python",
      html: "html",
      htm: "html",
      css: "css",
      scss: "css",
      less: "css",
      json: "json",
      md: "markdown",
      markdown: "markdown",
      toml: "config",
      yml: "config",
      yaml: "config",
      ini: "config",
      env: "config",
      sh: "shell",
      ps1: "shell",
      bash: "shell",
    }[extension] || "generic"
  );
}

function workspaceFileIcon(entry) {
  if (entry.kind === "directory") {
    return `
      <svg viewBox="0 0 24 24" aria-hidden="true" class="workspace-tree-icon workspace-tree-icon-folder">
        <path d="M3.75 7.5h5l1.7 2.05h9.8v6.9a1.8 1.8 0 0 1-1.8 1.8H5.55a1.8 1.8 0 0 1-1.8-1.8z"></path>
        <path d="M3.75 7.5V6.3a1.8 1.8 0 0 1 1.8-1.8h4.2l1.7 2.05"></path>
      </svg>
    `;
  }

  const iconType = workspaceIconType(entry.path || entry.name || "");
  const glyphs =
    {
      rust: '<circle cx="12" cy="13.2" r="2.25"></circle><path d="M12 9.4v1.55"></path><path d="M12 15.45V17"></path><path d="M8.95 11.2l1.1.62"></path><path d="M13.95 14.05l1.1.63"></path><path d="M8.95 15.2l1.1-.63"></path><path d="M13.95 12.35l1.1-.62"></path>',
      javascript: '<path d="M10.05 10.1v4.25c0 1.2-.62 1.95-1.82 1.95-.7 0-1.3-.22-1.76-.68"></path><path d="M12.85 14.9c.5.88 1.18 1.4 2.4 1.4 1 0 1.67-.48 1.67-1.18 0-.82-.64-1.1-1.72-1.58l-.58-.25c-1.68-.72-2.45-1.62-2.45-3.12 0-1.55 1.18-2.72 3.03-2.72 1.3 0 2.26.46 2.94 1.64"></path>',
      typescript: '<path d="M7.3 10.1h5.15"></path><path d="M9.88 10.1v6.2"></path><path d="M14.2 14.9c.48.88 1.08 1.4 2.08 1.4.88 0 1.48-.48 1.48-1.18 0-.82-.58-1.1-1.55-1.58l-.52-.25c-1.52-.72-2.2-1.62-2.2-3.12 0-1.55 1.08-2.72 2.8-2.72 1.2 0 2.08.46 2.72 1.64"></path>',
      python: '<path d="M9.15 9.2c0-.98.8-1.78 1.78-1.78h2.47c.98 0 1.78.8 1.78 1.78v1.65c0 .98-.8 1.78-1.78 1.78h-3.4c-.98 0-1.78.8-1.78 1.78v.38"></path><circle cx="13.8" cy="9.8" r=".55" class="workspace-tree-icon-dot"></circle><path d="M14.85 16.1c0 .98-.8 1.78-1.78 1.78H10.6c-.98 0-1.78-.8-1.78-1.78v-1.65c0-.98.8-1.78 1.78-1.78H14c.98 0 1.78-.8 1.78-1.78v-.38"></path><circle cx="10.2" cy="15.5" r=".55" class="workspace-tree-icon-dot"></circle>',
      html: '<path d="M9.1 10.5l-1.9 1.95 1.9 1.95"></path><path d="M14.9 10.5l1.9 1.95-1.9 1.95"></path><path d="M13.2 9.55l-2.4 5.8"></path>',
      css: '<path d="M8.25 10.4a2.5 2.5 0 0 1 2.5-2.5h4.1"></path><path d="M15.75 14.6a2.5 2.5 0 0 1-2.5 2.5h-4.1"></path><path d="M14.95 10.4a2.5 2.5 0 0 1 0 4.2"></path><path d="M9.05 14.6a2.5 2.5 0 0 1 0-4.2"></path>',
      json: '<path d="M9.6 8.95c-1.2.7-1.75 1.75-1.75 3.55s.55 2.85 1.75 3.55"></path><path d="M14.4 8.95c1.2.7 1.75 1.75 1.75 3.55s-.55 2.85-1.75 3.55"></path><path d="M12 10.6v.2"></path><path d="M12 12.5v.2"></path><path d="M12 14.4v.2"></path>',
      markdown: '<path d="M7.6 15.8v-6.1l2.2 2.8 2.2-2.8v6.1"></path><path d="M14.6 10.05v4.6"></path><path d="M14.6 14.65l1.45-1.45"></path><path d="M14.6 14.65l-1.45-1.45"></path><path d="M17.05 10.05h1.2"></path>',
      config: '<circle cx="12" cy="12.7" r="1.55"></circle><path d="M12 8.9v1.1"></path><path d="M12 15.4v1.1"></path><path d="M8.9 12.7H10"></path><path d="M14 12.7h1.1"></path><path d="M9.8 10.5l.78.78"></path><path d="M13.42 14.12l.78.78"></path><path d="M14.2 10.5l-.78.78"></path><path d="M10.58 14.12l-.78.78"></path>',
      shell: '<path d="M8.05 10.45l2.55 2.1-2.55 2.1"></path><path d="M12.6 15.3h3.35"></path>',
      generic: '<path d="M8.15 10.4h7.7"></path><path d="M8.15 12.55h7.7"></path><path d="M8.15 14.7h5.1"></path>',
    }[iconType] || '<path d="M8.15 10.4h7.7"></path><path d="M8.15 12.55h7.7"></path><path d="M8.15 14.7h5.1"></path>';

  return `
    <svg viewBox="0 0 24 24" aria-hidden="true" class="workspace-tree-icon workspace-tree-icon-file is-${iconType}">
      <path class="workspace-tree-icon-file-page" d="M8.05 4.75h6.9l3 3v10.5a1.8 1.8 0 0 1-1.8 1.8H8.05a1.8 1.8 0 0 1-1.8-1.8V6.55a1.8 1.8 0 0 1 1.8-1.8z"></path>
      <path class="workspace-tree-icon-file-fold" d="M14.95 4.75v3h3"></path>
      <g class="workspace-tree-icon-file-glyph">
        ${glyphs}
      </g>
    </svg>
  `;
}

function renderWorkspaceTreeNode(entry, depth = 0) {
  const rowClass = entry.kind === "directory" ? "workspace-tree-entry is-directory" : "workspace-tree-entry is-file";
  const isActive = entry.path === activeWorkspaceFilePath;
  const hasChildren = Array.isArray(entry.children) && entry.children.length;
  const isExpanded = entry.kind === "directory" ? expandedWorkspaceDirs.has(entry.path) : false;
  const childrenMarkup = hasChildren && isExpanded
    ? `<div class="workspace-tree-children">${entry.children.map((child) => renderWorkspaceTreeNode(child, depth + 1)).join("")}</div>`
    : "";

  return `
    <div class="workspace-tree-node">
      <button
        class="${rowClass}${isActive ? " is-active" : ""}"
        type="button"
        data-workspace-path="${escapeHtml(entry.path)}"
        data-workspace-kind="${escapeHtml(entry.kind)}"
        aria-expanded="${entry.kind === "directory" ? String(isExpanded) : "false"}"
        style="--tree-depth:${depth};"
      >
        ${
          entry.kind === "directory"
            ? `<span class="workspace-tree-caret${isExpanded ? " is-expanded" : ""}" aria-hidden="true"><svg viewBox="0 0 12 12"><path d="M4 2.75 7.75 6 4 9.25"></path></svg></span>`
            : `<span class="workspace-tree-caret workspace-tree-caret-file" aria-hidden="true"></span>`
        }
        ${workspaceFileIcon(entry)}
        <span class="workspace-tree-entry-label">${escapeHtml(entry.name)}</span>
      </button>
      ${childrenMarkup}
    </div>
  `;
}

function renderWorkspaceTree(browserData) {
  const data = browserData || {};
  workspaceTreeData = Array.isArray(data.entries) ? data.entries : [];

  if (workspaceFilesSubtitle) {
    workspaceFilesSubtitle.textContent = data.root_path || bootstrapData?.workspace_root || "";
  }

  if (!workspaceTree) return;

  if (!workspaceTreeData.length) {
    activeWorkspaceFilePath = null;
    renderWorkspaceFile(null);
    workspaceTree.innerHTML = `<div class="workspace-tree-empty">No files available.</div>`;
    renderWorkspaceTabs();
    return;
  }

  const files = workspaceFlattenFiles(workspaceTreeData, []);
  const dirPaths = collectWorkspaceDirs(workspaceTreeData, []);
  const nextExpanded = new Set([...expandedWorkspaceDirs].filter((path) => dirPaths.includes(path)));
  expandedWorkspaceDirs = nextExpanded;
  if (activeWorkspaceFilePath && !files.some((entry) => entry.path === activeWorkspaceFilePath)) {
    activeWorkspaceFilePath = null;
  }

  workspaceOpenTabs = workspaceOpenTabs.filter((path) => files.some((entry) => entry.path === path));

  workspaceTree.innerHTML = workspaceTreeData.map((entry) => renderWorkspaceTreeNode(entry, 0)).join("");
  renderWorkspaceTabs();

  workspaceTree.querySelectorAll("[data-workspace-path]").forEach((button) => {
    button.addEventListener("click", async (event) => {
      const nextPath = button.getAttribute("data-workspace-path") || "";
      const kind = button.getAttribute("data-workspace-kind") || "file";
      if (!nextPath) return;
      if (kind === "directory") {
        window.clearTimeout(workspaceFileClickTimer);
        workspaceFileClickTimer = null;
        if (expandedWorkspaceDirs.has(nextPath)) {
          expandedWorkspaceDirs.delete(nextPath);
        } else {
          expandedWorkspaceDirs.add(nextPath);
        }
        renderWorkspaceTree(browserData);
        return;
      }

      if (event.detail === 1) {
        window.clearTimeout(workspaceFileClickTimer);
        workspaceFileClickTimer = window.setTimeout(() => {
          activeWorkspaceFilePath = nextPath;
          expandWorkspacePathAncestors(nextPath);
          renderWorkspaceTree(browserData);
          workspaceFileClickTimer = null;
        }, 220);
        return;
      }

      window.clearTimeout(workspaceFileClickTimer);
      workspaceFileClickTimer = null;
      activeWorkspaceFilePath = nextPath;
      expandWorkspacePathAncestors(nextPath);
      renderWorkspaceTree(browserData);
      await loadWorkspaceFile(nextPath);
    });
  });
}

function renderWorkspaceFile(file, options = {}) {
  if (!workspaceCodeContent || !workspaceCodePath || !workspaceCodeMeta) return;
  if (!file) {
    isWorkspaceCodeOpen = false;
    syncWorkspaceDraftForCurrentFile();
    rememberWorkspaceRenderMode();
    currentWorkspaceFile = null;
    workspaceCodeRenderMode = "source";
    markWorkspaceEditorDirty(false);
    hideCodePanel();
    workspaceCodePath.textContent = "Select a file";
    workspaceCodeMeta.textContent = "";
    document.querySelector(".workspace-code-panel")?.classList.add("is-closed");
    if (workspaceLauncher) {
      workspaceLauncher.hidden = !preserveWorkspaceSlotWhenCodeClosed;
    }
    disposeWorkspaceMonaco();
    updateWorkspaceCodeView();
    return;
  }

  const preservePanelVisibility = Boolean(options.preservePanelVisibility);
  isWorkspaceCodeOpen = true;
  syncWorkspaceDraftForCurrentFile();
  rememberWorkspaceRenderMode();
  const wasSameFile = String(currentWorkspaceFile?.path || "") === String(file.path || "");
  const previousRenderMode = workspaceCodeRenderMode;
  currentWorkspaceFile = file;
  if (file.path && !workspaceOpenTabs.includes(file.path)) {
    workspaceOpenTabs.push(file.path);
  }
  workspaceCodeRenderMode = isMarkdownWorkspaceFile(file)
    ? (wasSameFile
        ? previousRenderMode
        : (workspaceRenderModeByPath.get(file.path) || "rendered"))
    : "source";
  markWorkspaceEditorDirty(workspaceFileDisplayContent(file) !== String(file.content || ""));
  ensureCodePanelVisible();
  document.querySelector(".workspace-code-panel")?.classList.remove("is-closed");
  if (workspaceLauncher) {
    workspaceLauncher.hidden = true;
  }
  workspaceCodePath.textContent = file.path || file.name || "";
  workspaceCodeMeta.textContent = workspaceFileMetaText(file);
  updateWorkspaceCodeView();
  scheduleWorkspaceMonacoLayout();
}

async function loadWorkspaceFile(path, options = {}) {
  const response = await hostClient.workspace.openFile(path);

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(errorText || `workspace file failed: ${response.status}`);
  }

  const payload = await response.json();
  const file = payload?.data?.file || null;
  cacheWorkspaceFile(file);
  renderWorkspaceFile(file, options);
  return file;
}

async function saveWorkspaceFile() {
  if (!currentWorkspaceFile || !workspaceCodeCanEdit(currentWorkspaceFile)) return;
  const nextContent = workspaceEditorText();
  if (nextContent === String(currentWorkspaceFile.content || "")) {
    workspaceDraftCache.delete(currentWorkspaceFile.path);
    renderWorkspaceTabs();
    markWorkspaceEditorDirty(false);
    return;
  }

  const response = await hostClient.workspace.saveFile(currentWorkspaceFile.path, nextContent);

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(errorText || `workspace file save failed: ${response.status}`);
  }

  const payload = await response.json();
  if (currentWorkspaceFile?.path) {
    workspaceDraftCache.delete(currentWorkspaceFile.path);
  }
  renderWorkspaceFile(payload?.data?.file || null);
  renderWorkspaceTabs();
  try {
    await loadBootstrap();
    if (activeActivityPanel === "git" || currentMainView === "git") {
      await loadGitState(currentGitFetchOptions(currentGitView));
    }
  } catch (_error) {
    // Ignore refresh failures after a successful save.
  }
  showToast(currentLanguage === "zh" ? "代码已保存" : "Code saved");
}

function reviewChipCount(file) {
  return `+${Number(file?.additions || 0)} / -${Number(file?.deletions || 0)}`;
}

function buildReviewFromMessages(messages) {
  if (bootstrapData?.review && Array.isArray(bootstrapData.review.files)) {
    return bootstrapData.review;
  }
  const files = [];
  const seen = new Map();
  const allMessages = Array.isArray(messages) ? messages : [];
  const lastUserIndex = [...allMessages]
    .map((message, index) => ({ message, index }))
    .filter(({ message }) => message && message.kind === "message" && message.role === "user")
    .map(({ index }) => index)
    .pop();
  const scopedMessages = lastUserIndex == null ? allMessages : allMessages.slice(lastUserIndex + 1);
  scopedMessages.forEach((message) => {
    if (!message || message.kind !== "diff" || !message.file_path) return;
    const path = String(message.file_path || "");
    const entry = {
      path,
      status: "edited",
      additions: Number(message.added || 0),
      deletions: Number(message.removed || 0),
    };
    seen.set(path, entry);
  });
  files.push(...seen.values());
  const total_additions = files.reduce((sum, file) => sum + Number(file.additions || 0), 0);
  const total_deletions = files.reduce((sum, file) => sum + Number(file.deletions || 0), 0);
  return {
    available: true,
    total_files: files.length,
    total_additions,
    total_deletions,
    files,
    error: null,
  };
}

function renderReviewDetail(detail, file) {
  if (!detail) {
    return `<div class="review-detail-empty">${escapeHtml(t("reviewLoading"))}</div>`;
  }

  const previewKind = String(detail.preview_kind || "").toLowerCase();
  const isBinary = Boolean(detail.is_binary);
  const reviewPath = String(detail.path || file?.path || "");
  const rawUrl = reviewPath ? hostClient.workspace.rawFileUrl(reviewPath) : "";
  const hunks = Array.isArray(detail.hunks) ? detail.hunks : [];
  const hunkMarkup = isBinary
    ? `
      <div class="review-artifact-card">
        <div class="review-artifact-meta">${escapeHtml(detail.mime_type || previewKind || "binary")}</div>
        ${
          previewKind === "image" && rawUrl
            ? `<img class="review-artifact-image" src="${escapeHtml(rawUrl)}" alt="${escapeHtml(reviewPath || "artifact")}" />`
            : previewKind === "pdf" && rawUrl
              ? `<iframe class="review-artifact-frame" src="${escapeHtml(rawUrl)}" title="${escapeHtml(reviewPath || "artifact")}"></iframe>`
              : `<div class="review-detail-empty">${escapeHtml(currentLanguage === "zh" ? "该产物为二进制文件，请在工作区预览中打开。" : "This artifact is binary. Open it in the workspace preview.")}</div>`
        }
      </div>
    `
    : hunks.length
    ? hunks
        .map((hunk) => {
          const rows = (hunk.lines || [])
            .map((line) => {
              const kind = String(line.kind || "context");
              return `
                <div class="review-code-row is-${escapeHtml(kind)}">
                  <span class="review-code-gutter">${line.old_number ?? ""}</span>
                  <span class="review-code-gutter">${line.new_number ?? ""}</span>
                  <span class="review-code-content">${escapeHtml(displayPlainText(line.content || "", line.content || ""))}</span>
                </div>
              `;
            })
            .join("");
          return `
            <section class="review-hunk">
              <div class="review-hunk-header">${escapeHtml(displayPlainText(hunk.header || "", hunk.header || ""))}</div>
              <div class="review-code">${rows}</div>
            </section>
          `;
        })
        .join("")
    : `<div class="review-detail-empty">${escapeHtml(t("reviewEmpty"))}</div>`;

  return `
    <div class="review-detail-head">
      <div class="review-detail-path">${escapeHtml(displayPlainText(detail.path || file?.path || "", detail.path || file?.path || ""))}</div>
      <div class="review-detail-meta">${escapeHtml(reviewChipCount(file || detail))}</div>
    </div>
    ${hunkMarkup}
  `;
}

function renderReview(review) {
  if (!reviewStrip) return;
  reviewStrip.hidden = false;
  reviewStrip.innerHTML = "";
  reviewStrip.hidden = true;
}

async function loadReviewFile(path) {
  const response = await hostClient.workspace.reviewFile(path);
  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(errorText || `review file failed: ${response.status}`);
  }
  const payload = await response.json();
  return payload?.data?.file || null;
}

async function ensureReviewDetail(path) {
  if (!path || reviewDetailCache.has(path)) {
    renderReview(bootstrapData?.review || null);
    return;
  }

  reviewDetailCache.set(path, null);
  renderReview(bootstrapData?.review || null);

  try {
    const detail = await loadReviewFile(path);
    reviewDetailCache.set(path, detail || { error: t("reviewError") });
  } catch (error) {
    console.error(error);
    reviewDetailCache.set(path, { error: t("reviewError") });
  }

  renderReview(bootstrapData?.review || null);
}

function sessionSummary(session) {
  const summary = clipDisplayText(session.summary, 42);
  if (summary) return summary;
  const title = clipDisplayText(session.title, 42) || t("sessionUntitled");
  if (normalizeText(title) === normalizeText(t("sessionUntitled")) || normalizeText(title) === normalizeText("New conversation")) {
    return currentLanguage === "zh" ? "新会话" : "New session";
  }
  return "";
}

function createSessionEntry(session, { emphasis = false } = {}) {
  const row = document.createElement("div");
  row.className = "session-row";
  const sessionTime = formatSessionTime(session.updated_at || session.created_at);
  const runState = getSessionRunState(session.id);
  const isRunning = Boolean(runState?.running);
  const isWaitingApproval = Boolean(runState?.waitingApproval);
  const displayTitle = clipDisplayText(session.title, emphasis ? 32 : 28) || t("sessionUntitled");
  const displaySummary = sessionSummary(session);

  const entry = document.createElement("button");
  entry.type = "button";
  entry.className = emphasis ? "session-entry is-emphasis" : "session-entry";
  entry.innerHTML = `
    <span class="session-head">
      <span class="session-name-wrap">
        <span class="session-name">${escapeHtml(displayTitle)}</span>
        ${isRunning ? `
          <span class="session-running-indicator${isWaitingApproval ? " is-waiting" : ""}" aria-hidden="true">
            <span></span><span></span><span></span>
          </span>
        ` : ""}
      </span>
      <span class="session-time">${escapeHtml(sessionTime)}</span>
    </span>
    ${displaySummary ? `<span class="session-meta">${escapeHtml(displaySummary)}</span>` : ""}
  `;
  entry.addEventListener("click", async () => {
    try {
      await switchSession(session.id);
    } catch (error) {
      console.error(error);
      showToast(t("toastSendFailed"));
    }
  });

  row.appendChild(entry);

  const actions = document.createElement("div");
  actions.className = "session-actions";
  actions.innerHTML = `
    <button class="session-menu-trigger" type="button" aria-label="${escapeHtml(t("settingsLabel"))}">
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <circle cx="12" cy="5" r="1.8"></circle>
        <circle cx="12" cy="12" r="1.8"></circle>
        <circle cx="12" cy="19" r="1.8"></circle>
      </svg>
    </button>
  `;
  const trigger = actions.querySelector(".session-menu-trigger");
  trigger?.addEventListener("click", (event) => {
    event.stopPropagation();
    openSessionMenu(session.id, trigger);
  });
  row.appendChild(actions);

  return row;
}

function renderCurrentSession(sessions, currentSessionId) {
  if (!currentSessionList) return;
  currentSessionList.innerHTML = "";
  const current =
    (sessions || []).find((session) => session.id === currentSessionId) ||
    (sessions || [])[0];
  if (!current) return;
  currentSessionList.appendChild(createSessionEntry(current, { emphasis: true }));
}

function renderSessionList(sessions, currentSessionId) {
  if (!sessionList) return;
  sessionList.innerHTML = "";
  const recents = (sessions || []).filter((session) => session.id !== currentSessionId);
  recents.forEach((session) => {
    sessionList.appendChild(createSessionEntry(session));
  });
}

function closeSessionMenus() {
  activeSessionMenuId = null;
  activeSessionMenuAnchor = null;
  if (sessionMenu) {
    sessionMenu.hidden = true;
  }
  document
    .querySelectorAll(".session-menu-trigger[aria-expanded='true']")
    .forEach((button) => button.setAttribute("aria-expanded", "false"));
}

function positionSessionMenu(anchor) {
  if (!anchor || !sessionMenu) return;
  const rect = anchor.getBoundingClientRect();
  const left = Math.max(12, rect.left - 100);
  const top = Math.min(window.innerHeight - 80, rect.bottom + 8);
  sessionMenu.style.left = `${left}px`;
  sessionMenu.style.top = `${top}px`;
}

function openSessionMenu(sessionId, anchor) {
  if (!sessionMenu || !anchor) return;
  if (activeSessionMenuId === sessionId && !sessionMenu.hidden) {
    closeSessionMenus();
    closePanelMenu();
    closePanelMenu();
    return;
  }
  closeSessionMenus();
  activeSessionMenuId = sessionId;
  activeSessionMenuAnchor = anchor;
  anchor.setAttribute("aria-expanded", "true");
  sessionMenu.hidden = false;
  positionSessionMenu(anchor);
}

function normalizePhaseIndex(research) {
  const explicit = Number(research?.phase_index || 0);
  if (explicit > 0) return explicit;
  const phaseName = normalizeText(research?.phase);
  const index = RESEARCH_PHASES.findIndex((phase) =>
    phase.aliases.some((alias) => normalizeText(alias) === phaseName),
  );
  return index >= 0 ? index + 1 : 1;
}

function researchSecurityClass(level) {
  const normalized = normalizeText(level);
  if (normalized.includes("strict")) return "is-strict";
  if (normalized.includes("confidential")) return "is-confidential";
  return "is-public";
}

function researchWorkflowLabel(kind) {
  const key = String(kind || "").trim().toLowerCase();
  const labels = {
    deep_learning: currentLanguage === "zh" ? "深度学习" : "Deep learning",
    experimental_design: currentLanguage === "zh" ? "实验设计" : "Experimental design",
    literature_review: currentLanguage === "zh" ? "文献综述" : "Literature review",
    simulation: currentLanguage === "zh" ? "仿真" : "Simulation",
    data_analysis: currentLanguage === "zh" ? "数据分析" : "Data analysis",
    adaptive_research: currentLanguage === "zh" ? "自适应研究" : "Adaptive research",
  };
  return labels[key] || (currentLanguage === "zh" ? "研究流程" : "Research workflow");
}

function researchStateLabel(state) {
  const key = String(state || "").trim().toLowerCase();
  const labels = {
    active: currentLanguage === "zh" ? "进行中" : "Active",
    blocked: currentLanguage === "zh" ? "已阻塞" : "Blocked",
    resumable: currentLanguage === "zh" ? "可恢复" : "Resumable",
    complete: currentLanguage === "zh" ? "已完成" : "Complete",
  };
  return labels[key] || (currentLanguage === "zh" ? "进行中" : "Active");
}

function resolveResearchOverallState(research) {
  const raw = String(research?.overall_state || "").trim().toLowerCase();
  const verifierStatus = String(research?.runtime?.verifier?.status || "").trim().toLowerCase();
  if (verifierStatus === "pass" || verifierStatus === "complete") {
    return "complete";
  }
  return raw || "active";
}

function researchGraphMarkup(research, options = {}) {
  const graph = research?.graph || {};
  const nodes = Array.isArray(graph.nodes) ? graph.nodes : [];
  const edges = Array.isArray(graph.edges) ? graph.edges : [];
  const nodeById = new Map(
    nodes.map((node) => [String(node?.id || ""), node]),
  );
  const canvasHeight = Math.max(options.minHeight || 260, ...nodes.map((node) => Number(node.y || 0) + 76), options.minHeight || 260);
  const edgeMarkup = edges
    .map((edge) => {
      const from = nodeById.get(String(edge?.from || ""));
      const to = nodeById.get(String(edge?.to || ""));
      if (!from || !to) return "";
      const x1 = Number(from.x || 0) + 108;
      const y1 = Number(from.y || 0) + 27;
      const x2 = Number(to.x || 0) + 16;
      const y2 = Number(to.y || 0) + 27;
      const midX = Math.round((x1 + x2) / 2);
      const path = `M ${x1} ${y1} C ${midX} ${y1}, ${midX} ${y2}, ${x2} ${y2}`;
      return `<path class="research-flow-edge" d="${escapeHtml(path)}"></path>`;
    })
    .join("");
  const nodeMarkup = nodes
    .map(
      (node) => `
        <div
          class="research-flow-node is-${escapeHtml(node.status || "pending")} lane-${escapeHtml(node.lane || "plan")}"
          style="--node-x:${Number(node.x || 0)}px;--node-y:${Number(node.y || 0)}px"
        >
          <div class="research-flow-node-title">${escapeHtml(node.label || "")}</div>
          <div class="research-flow-node-detail">${escapeHtml(node.detail || "")}</div>
        </div>
      `,
    )
    .join("");
  return {
    nodes,
    currentNode: nodes.find((node) => node.status === "current") || nodes[0] || null,
    html: `
      <div class="research-flow-board" style="height:${canvasHeight}px">
        <svg class="research-flow-edges" viewBox="0 0 280 ${canvasHeight}" preserveAspectRatio="none" aria-hidden="true">
          ${edgeMarkup}
        </svg>
        ${nodeMarkup}
      </div>
    `,
  };
}

function renderResearchLegacy(research) {
  if (!researchPanel) return;
  const currentIndex = normalizePhaseIndex(research);
  const topic =
    research?.topic && research.topic !== "General scientific inquiry"
      ? research.topic
      : t("researchTemplateTopic");
  const nextPhase = research?.next_phase || RESEARCH_PHASES[Math.min(currentIndex, RESEARCH_PHASES.length - 1)]?.key;
  const nextPhaseLabel =
    typeof nextPhase === "string" && nextPhase.startsWith("phase")
      ? t(nextPhase)
      : RESEARCH_PHASES.find((phase) =>
          phase.aliases.some((alias) => normalizeText(alias) === normalizeText(nextPhase)),
        )
          ? t(
              RESEARCH_PHASES.find((phase) =>
                phase.aliases.some((alias) => normalizeText(alias) === normalizeText(nextPhase)),
              ).key,
            )
          : nextPhase || t("researchStatusTemplate");

  researchPanel.innerHTML = `
    <div class="research-summary">
      <div class="research-topic">${escapeHtml(topic)}</div>
      <div class="research-meta">${escapeHtml(t("researchStatus"))}: ${escapeHtml(
        currentIndex > 0 ? t(RESEARCH_PHASES[currentIndex - 1].key) : t("researchStatusTemplate"),
      )} / ${currentIndex}/${research?.phase_total || RESEARCH_PHASES.length}</div>
      <div class="research-meta">${escapeHtml(t("researchNext"))}: ${escapeHtml(nextPhaseLabel)}</div>
      <div class="research-meta">${escapeHtml(t("researchAssessment"))}: ${escapeHtml(
        `${t("researchAssessmentGood")} ${t("researchAssessmentHuman")}`,
      )}</div>
    </div>
    <div class="research-phase-list">
      ${RESEARCH_PHASES.map((phase, index) => {
        const order = index + 1;
        const classes = [
          "research-phase-item",
          order < currentIndex ? "is-done" : "",
          order === currentIndex ? "is-current" : "",
        ]
          .filter(Boolean)
          .join(" ");
        return `
          <div class="${classes}">
            <span class="research-phase-dot"></span>
            <div class="research-phase-text">
              <div class="research-phase-name">${escapeHtml(t(phase.key))}</div>
              <div class="research-phase-note">${escapeHtml(t(phase.noteKey))}</div>
            </div>
          </div>
        `;
      }).join("")}
    </div>
    <div class="research-footer">
      <span class="research-pill ${researchSecurityClass(research?.security_level)}">${escapeHtml(
        `${t("researchSecurity")}: ${research?.security_level || "Public"}`,
      )}</span>
      ${
        research?.competition_mode
          ? `<span class="research-pill">${escapeHtml(t("researchCompetition"))}</span>`
          : ""
      }
      ${
        research?.waiting_approval
          ? `<span class="research-pill is-confidential">${escapeHtml(t("researchWaiting"))}</span>`
          : ""
      }
    </div>
    <div class="research-meta">${escapeHtml(t("researchWorkspace"))}: ${escapeHtml(
      research?.workspace || bootstrapData?.workspace_root || "",
    )}</div>
  `;
}

function renderResearchRuntimeSubagents(items, options = {}) {
  const subagents = Array.isArray(items) ? items : [];
  const limit = Number.isFinite(options.limit) ? options.limit : null;
  const visible = limit != null ? subagents.slice(-limit) : subagents;
  if (!visible.length) return "";
  return `
    <div class="research-runtime-subagents">
      ${visible
        .map((item) => `
          <details class="research-runtime-card"${String(item.status || "").toLowerCase() === "running" ? " open" : ""}>
            <summary class="research-runtime-summary">
              <span class="research-runtime-name">${escapeHtml(cleanDisplayText(item.name || "subagent", currentLanguage === "zh" ? "子代理" : "subagent"))}</span>
              <span class="research-runtime-pill">${escapeHtml(renderDelegateStatus(item.status || ""))}</span>
            </summary>
            <div class="research-runtime-body">
              ${cleanDisplayText(item.purpose) ? `<div class="research-runtime-line"><span>${escapeHtml(zhLabel("目的", "Purpose"))}</span><strong>${escapeHtml(cleanDisplayText(item.purpose))}</strong></div>` : ""}
              ${cleanDisplayText(item.input) ? `<div class="research-runtime-line"><span>${escapeHtml(zhLabel("输入", "Input"))}</span><strong>${escapeHtml(cleanDisplayText(item.input))}</strong></div>` : ""}
              ${cleanDisplayText(String(item.output || "").slice(0, options.outputLimit || 220)) ? `<div class="research-runtime-line"><span>${escapeHtml(zhLabel("输出", "Output"))}</span><strong>${escapeHtml(cleanDisplayText(String(item.output || "").slice(0, options.outputLimit || 220)))}</strong></div>` : ""}
              ${
                Array.isArray(item.evidence) && item.evidence.length
                  ? `<div class="research-runtime-list">${cleanDisplayList(item.evidence.slice(0, options.evidenceLimit || 4)).map((entry) => `<div class="research-review-item">${escapeHtml(entry)}</div>`).join("")}</div>`
                  : ""
              }
            </div>
          </details>
        `)
        .join("")}
    </div>
  `;
}

function renderResearchRuntimeVerifier(report, options = {}) {
  if (!report) return "";
  const statusKey = String(report.status || "").toLowerCase();
  const shouldOpen =
    Boolean(options.forceOpen) ||
    statusKey === "running" ||
    statusKey === "repair" ||
    statusKey === "failed";
  return `
    <details class="research-runtime-card research-runtime-verifier"${shouldOpen ? " open" : ""}>
      <summary class="research-runtime-summary">
        <span class="research-runtime-name">${escapeHtml(currentLanguage === "zh" ? "验证器" : "Verifier")}</span>
        <span class="research-runtime-pill">${escapeHtml(renderDelegateStatus(report.status || ""))}</span>
      </summary>
      <div class="research-runtime-body">
        ${cleanDisplayText(report.summary) ? `<div class="research-runtime-line"><span>${escapeHtml(currentLanguage === "zh" ? "摘要" : "Summary")}</span><strong>${escapeHtml(cleanDisplayText(report.summary))}</strong></div>` : ""}
        ${
          Array.isArray(report.checks) && report.checks.length
            ? `<div class="research-runtime-list">${report.checks.slice(0, options.checkLimit || 6).map((item) => {
                const title = cleanDisplayText(item.title || "check", "check");
                const detail = cleanDisplayText(item.detail || "");
                return `<div class="research-review-item">${escapeHtml(title)} [${escapeHtml(item.status || "")}]${detail ? ` - ${escapeHtml(detail)}` : ""}</div>`;
              }).join("")}</div>`
            : ""
        }
        ${
          Array.isArray(report.issues) && report.issues.length
            ? `<div class="research-runtime-list">${cleanDisplayList(report.issues.slice(0, options.issueLimit || 4)).map((item) => `<div class="research-review-item">${escapeHtml(item)}</div>`).join("")}</div>`
            : ""
        }
      </div>
    </details>
  `;
}

function renderRuntimeTimeline(items, options = {}) {
  const timeline = Array.isArray(items) ? items : [];
  if (!timeline.length) return "";
  const limit = Number.isFinite(options.limit) ? options.limit : null;
  const visible = limit != null ? timeline.slice(-limit) : timeline;
  const title = options.title || (currentLanguage === "zh" ? "运行时间线" : "Runtime timeline");
  const shouldOpen =
    Boolean(options.forceOpen) ||
    visible.some((item) => ["running", "repair", "failed"].includes(String(item?.status || "").toLowerCase()));
  return `
    <details class="research-runtime-card research-runtime-timeline"${shouldOpen ? " open" : ""}>
      <summary class="research-runtime-summary">
        <span class="research-runtime-name">${escapeHtml(title)}</span>
        <span class="research-runtime-pill">${escapeHtml(String(visible.length))}</span>
      </summary>
      <div class="research-runtime-body">
        <div class="research-runtime-timeline">
          ${visible
            .map((item) => {
              const status = String(item?.status || "complete").toLowerCase();
              const kind = String(item?.kind || "activity").toLowerCase();
              const rawTs = String(item?.ts || "");
              const shortTs = rawTs ? rawTs.replace("T", " ").slice(5, 16) : "";
              const titleText = cleanDisplayText(
                item?.title || item?.agent || "event",
                currentLanguage === "zh" ? "事件" : "event",
              );
              const detailText = cleanDisplayText(item?.detail || "");
              return `
                <div class="research-timeline-item is-${escapeHtml(status)} is-${escapeHtml(kind)}">
                  <div class="research-timeline-rail" aria-hidden="true"><span class="research-timeline-dot"></span></div>
                  <div class="research-timeline-main">
                    <div class="research-timeline-head">
                      <span class="research-timeline-title">${escapeHtml(titleText)}</span>
                      ${shortTs ? `<span class="research-timeline-time">${escapeHtml(shortTs)}</span>` : ""}
                    </div>
                    ${detailText ? `<div class="research-timeline-detail">${escapeHtml(detailText)}</div>` : ""}
                  </div>
                </div>
              `;
            })
            .join("")}
        </div>
      </div>
    </details>
  `;
}

function reviewerFeedbackSessionId() {
  return String(bootstrapData?.current_session_id || "").trim();
}

function currentReviewerFeedbackDraft(sessionId = reviewerFeedbackSessionId()) {
  const id = String(sessionId || "").trim();
  if (!id) {
    return {
      reviewer: "",
      score: "",
      comment: "",
      linkedRunId: "",
    };
  }
  if (!reviewerFeedbackDrafts.has(id)) {
    reviewerFeedbackDrafts.set(id, {
      reviewer: "",
      score: "",
      comment: "",
      linkedRunId: "",
    });
  }
  return reviewerFeedbackDrafts.get(id);
}

function syncReviewerFeedbackDraftFromResearch(research) {
  const sessionId = reviewerFeedbackSessionId();
  if (!sessionId) return;
  const draft = currentReviewerFeedbackDraft(sessionId);
  const currentRunId = cleanDisplayText(research?.reviewer_feedback?.current_run_id || "");
  if (currentRunId && !String(draft.linkedRunId || "").trim()) {
    draft.linkedRunId = currentRunId;
  }
}

function resetReviewerFeedbackDraft(sessionId = reviewerFeedbackSessionId(), research = bootstrapData?.research || null) {
  const id = String(sessionId || "").trim();
  if (!id) return;
  const nextRunId = cleanDisplayText(research?.reviewer_feedback?.current_run_id || "");
  reviewerFeedbackDrafts.set(id, {
    reviewer: "",
    score: "",
    comment: "",
    linkedRunId: nextRunId,
  });
}

function renderConversationPaperPdfEntry(research) {
  if (currentWorkspaceMode !== "research") return "";
  const paperWorkflow = research?.paper_workflow || null;
  const pdfPath = cleanDisplayText(paperWorkflow?.paper_pdf_path || "", "");
  const pdfStatus = cleanDisplayText(paperWorkflow?.pdf_compile_status || "", "");
  if (!pdfPath || normalizeText(pdfStatus) !== "compiled") return "";
  const sourceRunId = cleanDisplayText(paperWorkflow?.source_run_id || "", "");
  const summary = cleanDisplayText(
    paperWorkflow?.summary || research?.summary || "",
    zhLabel("论文 PDF 已可打开。", "Paper PDF is ready to open."),
  ) || zhLabel("论文 PDF 已可打开。", "Paper PDF is ready to open.");
  const statusLabel = paperPdfStatusLabel(pdfStatus);
  const runLabel = sourceRunId
    ? zhLabel(`来源运行 ${sourceRunId}`, `Source run ${sourceRunId}`)
    : zhLabel("研究产物", "Research deliverable");
  return `
    <section class="conversation-paper-entry" aria-label="${escapeHtml(zhLabel("论文 PDF 条目", "Paper PDF entry"))}">
      <div class="conversation-paper-entry-head">
        <span class="conversation-paper-entry-kicker">${escapeHtml(zhLabel("论文输出", "Paper output"))}</span>
        <span class="paper-workflow-pill ${escapeHtml(paperPdfStatusClass(pdfStatus))}">${escapeHtml(statusLabel)}</span>
      </div>
      <button
        class="conversation-paper-entry-card"
        type="button"
        data-open-workspace-file="${escapeHtml(pdfPath)}"
      >
        <span class="conversation-paper-entry-main">
          <strong>${escapeHtml(zhLabel("打开论文 PDF", "Open paper PDF"))}</strong>
          <span>${escapeHtml(summary)}</span>
          <span class="conversation-paper-entry-path">${escapeHtml(basename(pdfPath))}</span>
        </span>
        <span class="conversation-paper-entry-meta">${escapeHtml(runLabel)}</span>
      </button>
    </section>
  `;
}

function renderConversationPaperWorkflowPrompt(research) {
  if (currentWorkspaceMode !== "research") return "";
  if (!shouldShowPaperWorkflowPrompt(research)) return "";
  const sessionId = String(bootstrapData?.current_session_id || "").trim();
  const pending = Boolean(sessionId) && paperWorkflowPendingSessions.has(sessionId);
  const primaryLabel = pending ? t("paperWorkflowRunning") : t("paperWorkflowPromptGenerate");
  const metaLabel = zhLabel("研究闭环完成", "Research loop complete");
  return `
    <section class="conversation-paper-entry" aria-label="${escapeHtml(t("paperWorkflowPromptTitle"))}">
      <div class="conversation-paper-entry-card conversation-paper-prompt-card">
        <div class="conversation-paper-entry-head">
          <span class="conversation-paper-entry-kicker">${escapeHtml(t("paperWorkflowPromptTitle"))}</span>
          <span class="paper-workflow-pill is-ready">${escapeHtml(metaLabel)}</span>
        </div>
        <div class="conversation-paper-entry-main">
          <strong>${escapeHtml(t("paperWorkflowPromptReady"))}</strong>
          <span>${escapeHtml(t("paperWorkflowPromptHint"))}</span>
        </div>
        <div class="conversation-paper-prompt-actions">
          <button
            class="paper-workflow-run"
            type="button"
            data-paper-workflow-run="true"
            ${pending ? "disabled" : ""}
          >${escapeHtml(primaryLabel)}</button>
          <button
            class="paper-workflow-link conversation-paper-prompt-dismiss"
            type="button"
            data-paper-workflow-dismiss="${escapeHtml(sessionId)}"
          >${escapeHtml(t("paperWorkflowPromptLater"))}</button>
        </div>
      </div>
    </section>
  `;
}

function updateReviewerFeedbackDraft(field, value, sessionId = reviewerFeedbackSessionId()) {
  const id = String(sessionId || "").trim();
  if (!id) return;
  const draft = currentReviewerFeedbackDraft(id);
  draft[field] = value;
}

function reviewerFeedbackStateText(entry) {
  return entry?.resolved ? t("reviewerFeedbackResolved") : t("reviewerFeedbackOpen");
}

function reviewerFeedbackStateClass(entry) {
  return entry?.resolved ? "resolved" : "open";
}

function formatReviewerFeedbackScore(score) {
  if (score == null || Number.isNaN(Number(score))) return "";
  return String(Number(score));
}

function normalizeReviewerFeedbackPayload(payload) {
  const feedback = payload?.reviewer_feedback || payload || {};
  return {
    session_id: cleanDisplayText(feedback.session_id || ""),
    current_run_id: cleanDisplayText(feedback.current_run_id || ""),
    unresolved_count: Number(feedback.unresolved_count || 0),
    entries: Array.isArray(feedback.entries)
      ? feedback.entries.map((entry) => ({
          reviewer: cleanDisplayText(entry?.reviewer || ""),
          linked_run_id: cleanDisplayText(entry?.linked_run_id || ""),
          score: entry?.score == null ? null : Number(entry.score),
          comment: cleanDisplayText(entry?.comment || ""),
          resolved: Boolean(entry?.resolved),
          created_at: cleanDisplayText(entry?.created_at || ""),
          resolved_at: cleanDisplayText(entry?.resolved_at || ""),
        }))
      : [],
  };
}

function commitReviewerFeedbackPayload(payload, { render = true } = {}) {
  const normalized = normalizeReviewerFeedbackPayload(payload);
  bootstrapData = {
    ...(bootstrapData || {}),
    research: {
      ...(bootstrapData?.research || {}),
      reviewer_feedback: normalized,
    },
  };
  syncReviewerFeedbackDraftFromResearch(bootstrapData.research);
  if (render) {
    renderResearch(bootstrapData?.research || null);
  }
  return normalized;
}

async function refreshReviewerFeedback(options = {}) {
  const { silent = false } = options;
  const sessionId = await ensureSessionReady();
  if (!sessionId || reviewerFeedbackPendingSessions.has(sessionId)) {
    return bootstrapData?.research?.reviewer_feedback || null;
  }
  reviewerFeedbackPendingSessions.add(sessionId);
  try {
    const response = await hostClient.reviewerFeedback.state();
    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(errorText || `reviewer feedback state failed: ${response.status}`);
    }
    const payload = await response.json();
    const next = commitReviewerFeedbackPayload(payload?.data || payload);
    if (!silent) {
      showToast(t("toastReviewerFeedbackRefreshed"));
    }
    return next;
  } finally {
    reviewerFeedbackPendingSessions.delete(sessionId);
  }
}

async function submitReviewerFeedback() {
  const sessionId = await ensureSessionReady();
  const draft = currentReviewerFeedbackDraft(sessionId);
  const reviewer = String(draft.reviewer || "").trim();
  const comment = String(draft.comment || "").trim();
  const linkedRunId = String(draft.linkedRunId || "").trim();
  const scoreText = String(draft.score || "").trim();

  if (!reviewer || !comment) {
    showToast(t("reviewerFeedbackValidation"));
    return;
  }

  let score = null;
  if (scoreText) {
    score = Number(scoreText);
    if (!Number.isFinite(score) || score < 0 || score > 100) {
      showToast(t("reviewerFeedbackScoreInvalid"));
      return;
    }
    score = Math.round(score);
  }

  const response = await hostClient.reviewerFeedback.add({
    reviewer,
    linked_run_id: linkedRunId || null,
    score,
    comment,
  });
  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(errorText || `reviewer feedback add failed: ${response.status}`);
  }
  const payload = await response.json();
  commitReviewerFeedbackPayload(payload?.data || payload);
  resetReviewerFeedbackDraft(sessionId, bootstrapData?.research || null);
  renderResearch(bootstrapData?.research || null);
  showToast(t("toastReviewerFeedbackSaved"));
}

async function resolveReviewerFeedback(index) {
  const response = await hostClient.reviewerFeedback.resolve(index);
  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(errorText || `reviewer feedback resolve failed: ${response.status}`);
  }
  const payload = await response.json();
  commitReviewerFeedbackPayload(payload?.data || payload);
  showToast(t("toastReviewerFeedbackResolved"));
}

function renderReviewerFeedbackPanel(reviewerFeedback, options = {}) {
  const compact = options.compact === true;
  const allowForm = options.allowForm !== false;
  const entries = Array.isArray(reviewerFeedback?.entries) ? reviewerFeedback.entries : [];
  const unresolvedCount = Number(reviewerFeedback?.unresolved_count || 0);
  const currentRunId = cleanDisplayText(reviewerFeedback?.current_run_id || "");
  const sessionId = reviewerFeedbackSessionId();
  const draft = currentReviewerFeedbackDraft(sessionId);
  if (currentRunId && !String(draft.linkedRunId || "").trim()) {
    draft.linkedRunId = currentRunId;
  }
  const visibleEntries = compact ? entries.slice(0, 2) : entries;
  const metaText = template("reviewerFeedbackMeta", {
    count: unresolvedCount,
    total: entries.length,
  });
  const panelClass = compact ? "reviewer-feedback-panel is-compact" : "reviewer-feedback-panel";
  const currentRunLine = currentRunId
    ? `
      <div class="reviewer-feedback-current-run">
        <span>${escapeHtml(t("reviewerFeedbackCurrentRun"))}</span>
        <strong>${escapeHtml(currentRunId)}</strong>
      </div>
    `
    : "";
  const listMarkup = visibleEntries.length
    ? `
      <div class="reviewer-feedback-list">
        ${visibleEntries
          .map((entry, index) => `
            <article class="reviewer-feedback-item is-${escapeHtml(reviewerFeedbackStateClass(entry))}">
              <div class="reviewer-feedback-item-head">
                <div class="reviewer-feedback-item-main">
                  <strong>${escapeHtml(entry.reviewer || t("reviewerFeedbackReviewer"))}</strong>
                  ${entry.linked_run_id ? `<span class="reviewer-feedback-item-run">${escapeHtml(entry.linked_run_id)}</span>` : ""}
                </div>
                <div class="reviewer-feedback-item-side">
                  ${entry.score != null ? `<span class="reviewer-feedback-score">${escapeHtml(formatReviewerFeedbackScore(entry.score))}</span>` : ""}
                  <span class="reviewer-feedback-badge is-${escapeHtml(reviewerFeedbackStateClass(entry))}">${escapeHtml(reviewerFeedbackStateText(entry))}</span>
                </div>
              </div>
              <div class="reviewer-feedback-comment">${escapeHtml(entry.comment || "")}</div>
              ${
                !compact
                  ? `
                    <div class="reviewer-feedback-item-meta">
                      ${entry.created_at ? `<span>${escapeHtml(formatSessionTime(entry.created_at))}</span>` : "<span></span>"}
                      ${entry.resolved ? "" : `<button class="reviewer-feedback-action" type="button" data-reviewer-feedback-resolve="${index}">${escapeHtml(t("reviewerFeedbackResolve"))}</button>`}
                    </div>
                  `
                  : ""
              }
            </article>
          `)
          .join("")}
      </div>
    `
    : `<div class="reviewer-feedback-empty">${escapeHtml(t("reviewerFeedbackEmpty"))}</div>`;

  const formMarkup = allowForm && !compact
    ? `
      <form class="reviewer-feedback-form" data-reviewer-feedback-form="true">
        <div class="reviewer-feedback-form-head">
          <div class="reviewer-feedback-hint">${escapeHtml(t("reviewerFeedbackDraftHint"))}</div>
          <button class="reviewer-feedback-refresh" type="button" data-reviewer-feedback-refresh="true">${escapeHtml(t("reviewerFeedbackRefresh"))}</button>
        </div>
        <div class="reviewer-feedback-form-grid">
          <label class="reviewer-feedback-field">
            <span>${escapeHtml(t("reviewerFeedbackReviewer"))}</span>
            <input type="text" value="${escapeHtml(draft.reviewer || "")}" data-reviewer-feedback-field="reviewer" />
          </label>
          <label class="reviewer-feedback-field">
            <span>${escapeHtml(t("reviewerFeedbackScore"))}</span>
            <input type="number" min="0" max="100" placeholder="${escapeHtml(t("reviewerFeedbackScoreHint"))}" value="${escapeHtml(String(draft.score || ""))}" data-reviewer-feedback-field="score" />
          </label>
          <label class="reviewer-feedback-field reviewer-feedback-field-wide">
            <span>${escapeHtml(t("reviewerFeedbackRunId"))}</span>
            <input type="text" value="${escapeHtml(draft.linkedRunId || currentRunId || "")}" data-reviewer-feedback-field="linkedRunId" />
          </label>
          <label class="reviewer-feedback-field reviewer-feedback-field-wide">
            <span>${escapeHtml(t("reviewerFeedbackComment"))}</span>
            <textarea rows="3" data-reviewer-feedback-field="comment">${escapeHtml(draft.comment || "")}</textarea>
          </label>
        </div>
        <div class="reviewer-feedback-form-actions">
          <button class="reviewer-feedback-submit" type="submit">${escapeHtml(t("reviewerFeedbackAdd"))}</button>
        </div>
      </form>
    `
    : "";

  const actionBar = compact
    ? `<div class="reviewer-feedback-compact-actions"><button class="reviewer-feedback-refresh" type="button" data-reviewer-feedback-refresh="true">${escapeHtml(t("reviewerFeedbackRefresh"))}</button></div>`
    : "";

  return `
    <section class="${panelClass}">
      <div class="reviewer-feedback-head">
        <div>
          <div class="reviewer-feedback-title">${escapeHtml(t("reviewerFeedbackTitle"))}</div>
          <div class="reviewer-feedback-meta">${escapeHtml(metaText)}</div>
        </div>
      </div>
      ${currentRunLine}
      ${listMarkup}
      ${actionBar}
      ${formMarkup}
    </section>
  `;
}

function pushPaperWorkflowArtifact(items, seen, label, kind, rawPath) {
  const path = cleanDisplayText(rawPath || "", "");
  if (!path) return;
  const normalizedPath = path.replace(/\\/g, "/");
  if (seen.has(normalizedPath)) return;
  seen.add(normalizedPath);
  items.push({
    label: cleanDisplayText(label || "", zhLabel("论文产物", "Paper artifact")) || zhLabel("论文产物", "Paper artifact"),
    kind: cleanDisplayText(kind || "", ""),
    path,
  });
}

function paperWorkflowArtifacts(research) {
  const paperWorkflow = research?.paper_workflow || null;
  const items = [];
  const seen = new Set();
  if (Array.isArray(paperWorkflow?.artifacts)) {
    paperWorkflow.artifacts.forEach((artifact) => {
      pushPaperWorkflowArtifact(
        items,
        seen,
        artifact?.label || "",
        artifact?.kind || "",
        artifact?.path || "",
      );
    });
  }
  pushPaperWorkflowArtifact(items, seen, zhLabel("论文 PDF", "Paper PDF"), "pdf", paperWorkflow?.paper_pdf_path || "");
  pushPaperWorkflowArtifact(items, seen, zhLabel("论文 LaTeX", "Paper LaTeX"), "latex", paperWorkflow?.paper_latex_path || "");
  pushPaperWorkflowArtifact(items, seen, zhLabel("论文 Markdown", "Paper Markdown"), "markdown", paperWorkflow?.paper_markdown_path || "");
  pushPaperWorkflowArtifact(items, seen, zhLabel("参考文献 BibTeX", "References BibTeX"), "bibtex", paperWorkflow?.references_bib_path || "");
  pushPaperWorkflowArtifact(items, seen, zhLabel("产物附录", "Artifact Appendix"), "markdown", paperWorkflow?.appendix_markdown_path || "");
  pushPaperWorkflowArtifact(items, seen, zhLabel("结果包", "Result Bundle"), "json", paperWorkflow?.result_bundle_path || "");
  pushPaperWorkflowArtifact(items, seen, zhLabel("审查回复", "Review Response"), "json", paperWorkflow?.review_response_path || "");
  pushPaperWorkflowArtifact(items, seen, zhLabel("修订执行计划", "Revision Execution Plan"), "json", paperWorkflow?.revision_execution_plan_path || "");
  pushPaperWorkflowArtifact(items, seen, zhLabel("Workflow Checkpoint", "Workflow Checkpoint"), "json", paperWorkflow?.workflow_checkpoint_path || "");
  pushPaperWorkflowArtifact(items, seen, zhLabel("Rebuttal", "Reviewer Rebuttal"), "markdown", paperWorkflow?.rebuttal_markdown_path || "");
  pushPaperWorkflowArtifact(items, seen, zhLabel("论文章节", "Paper Sections"), "json", paperWorkflow?.section_bundle_path || "");
  pushPaperWorkflowArtifact(items, seen, zhLabel("修订前稿件", "Manuscript Before"), "json", paperWorkflow?.manuscript_bundle_before_path || "");
  pushPaperWorkflowArtifact(items, seen, zhLabel("修订后稿件", "Manuscript After"), "json", paperWorkflow?.manuscript_bundle_after_path || "");
  pushPaperWorkflowArtifact(items, seen, zhLabel("稿件 Diff", "Manuscript Diff"), "json", paperWorkflow?.manuscript_diff_path || "");
  pushPaperWorkflowArtifact(items, seen, zhLabel("论文打包", "Paper Bundle"), "json", paperWorkflow?.payload_path || "");
  return items;
}

function paperWorkflowPrimaryPath(research) {
  const paperWorkflow = research?.paper_workflow || null;
  return cleanDisplayText(
    paperWorkflow?.paper_pdf_path
      || paperWorkflow?.paper_latex_path
      || paperWorkflow?.paper_markdown_path
      || "",
    "",
  );
}

function paperWorkflowArtifactKindLabel(kind) {
  const key = normalizeText(kind);
  const labels = {
    markdown: "Markdown",
    latex: "LaTeX",
    pdf: "PDF",
    bibtex: "BibTeX",
    json: "JSON",
  };
  return labels[key] || cleanDisplayText(kind || "", zhLabel("文件", "File")) || zhLabel("文件", "File");
}

function paperWorkflowLifecycleBadge(research) {
  const workflow = research?.paper_workflow || null;
  const ready = research?.paper_ready || null;
  if (!workflow) {
    return {
      label: zhLabel("草稿待生成", "Draft pending"),
      className: "is-pending",
    };
  }
  if (ready?.ready) {
    return {
      label: zhLabel("论文已就绪", "Paper ready"),
      className: "is-ready",
    };
  }
  return {
    label: zhLabel("需要修订", "Needs revision"),
    className: "is-attention",
  };
}

function paperRevisionModeLabel(mode) {
  const key = normalizeText(mode);
  const labels = {
    fresh_draft: zhLabel("初始草稿", "Initial draft"),
    reviewer_guided_revision: zhLabel("审稿意见引导修订", "Reviewer-guided revision"),
    targeted_revision: zhLabel("鐎规艾鎮滄穱顔款吂", "Targeted revision"),
    force_rewrite: zhLabel("瀵搫鍩楅柌宥呭晸", "Forced rewrite"),
    rewrite: zhLabel("重写", "Rewrite"),
  };
  return labels[key] || cleanDisplayText(mode || "", zhLabel("修订", "Revision"));
}

function paperPdfStatusClass(status) {
  const key = normalizeText(status);
  if (key === "compiled") return "is-ready";
  if (key === "missing_toolchain") return "is-pending";
  if (key === "failed" || key === "error") return "is-attention";
  return "is-neutral";
}

function paperPdfStatusLabel(status) {
  const key = normalizeText(status);
  const labels = {
    compiled: zhLabel("PDF 已编译", "PDF compiled"),
    missing_toolchain: zhLabel("缺少 TeX 工具链", "TeX toolchain missing"),
    failed: zhLabel("PDF 编译失败", "PDF compile failed"),
    error: zhLabel("PDF 编译失败", "PDF compile failed"),
    pending: zhLabel("PDF 编译中", "PDF compiling"),
    running: zhLabel("PDF 编译中", "PDF compiling"),
  };
  return labels[key] || cleanDisplayText(status || "", zhLabel("PDF 状态未知", "PDF status unknown"));
}

function paperWorkflowTriggerLabel(mode) {
  const key = normalizeText(mode);
  const labels = {
    manual: zhLabel("手动", "Manual"),
    auto_finalize: zhLabel("自动收尾", "Auto finalize"),
    auto_bootstrap: zhLabel("自动引导", "Auto bootstrap"),
    auto_feedback_add: zhLabel("收到反馈后自动触发", "Auto on feedback add"),
    auto_feedback_resolve: zhLabel("反馈解决后自动触发", "Auto on feedback resolve"),
  };
  if (labels[key]) return labels[key];
  if (key.startsWith("auto")) return zhLabel("自动", "Auto");
  return cleanDisplayText(mode || "", zhLabel("未知触发器", "Unknown trigger"));
}

function renderPaperWorkflowMetaRows(rows) {
  const visibleRows = rows.filter((row) => row && cleanDisplayText(row.value || "", ""));
  if (!visibleRows.length) return "";
  return `
    <div class="paper-workflow-meta-list">
      ${visibleRows.map((row) => `
        <div class="paper-workflow-meta-row">
          <span class="paper-workflow-meta-label">${escapeHtml(row.label)}</span>
          <span class="paper-workflow-meta-value">${escapeHtml(cleanDisplayText(row.value || "", ""))}</span>
        </div>
      `).join("")}
    </div>
  `;
}

function renderPaperRevisionQueue(paperWorkflow, compact = false) {
  const queueSize = Number(paperWorkflow?.revision_queue_size || 0) || 0;
  const preview = Array.isArray(paperWorkflow?.revision_queue_preview)
    ? paperWorkflow.revision_queue_preview
      .map((item) => cleanDisplayText(item || "", ""))
      .filter(Boolean)
    : [];
  if (!queueSize && !preview.length) return "";
  const visibleItems = compact ? preview.slice(0, 2) : preview.slice(0, 4);
  return `
    <div class="paper-revision-queue">
      <div class="paper-revision-queue-head">
        <span class="paper-revision-queue-title">${escapeHtml(zhLabel("修订队列", "Revision queue"))}</span>
        <span class="paper-revision-queue-count">${escapeHtml(
          queueSize > 0
            ? zhLabel(`${queueSize} 项排队中`, `${queueSize} queued`)
            : zhLabel("没有排队中的修改", "No queued edits"),
        )}</span>
      </div>
      ${visibleItems.length ? `
        <div class="paper-revision-queue-list">
          ${visibleItems.map((item) => `
            <div class="paper-revision-queue-item">${escapeHtml(item)}</div>
          `).join("")}
        </div>
      ` : ""}
    </div>
  `;
}

function paperWorkflowGateChecks(paperWorkflow) {
  return Array.isArray(paperWorkflow?.paper_ready_gate?.manuscript_evidence_coverage?.checks)
    ? paperWorkflow.paper_ready_gate.manuscript_evidence_coverage.checks
    : [];
}

function paperWorkflowClaimGateChecks(paperWorkflow) {
  const coverage = paperWorkflow?.paper_ready_gate?.manuscript_evidence_coverage || null;
  if (Array.isArray(coverage?.claim_evidence_semantics?.checks)) {
    return coverage.claim_evidence_semantics.checks;
  }
  return Array.isArray(coverage?.checks)
    ? coverage.checks
      .find((item) => normalizeText(item?.check_id || "") === "claim_evidence_semantic_alignment")
      ?.evidence?.claim_evidence_gate?.checks || []
    : [];
}

function paperWorkflowProgressPrompts(research) {
  const paperWorkflow = research?.paper_workflow || null;
  const source = Array.isArray(paperWorkflow?.progress_prompts)
    ? paperWorkflow.progress_prompts
    : Array.isArray(research?.progress_prompts)
      ? research.progress_prompts
      : [];
  return source
    .map((item) => ({
      stageId: cleanDisplayText(item?.stage_id || "", ""),
      stageLabel: cleanDisplayText(item?.stage_label || "", ""),
      message: cleanDisplayText(item?.message || "", ""),
      evidenceContract: cleanDisplayList(item?.evidence_contract || []),
    }))
    .filter((item) => item.message || item.stageLabel);
}

function paperWorkflowClosureChecks(research) {
  const paperWorkflow = research?.paper_workflow || null;
  const source = Array.isArray(paperWorkflow?.closure_checks)
    ? paperWorkflow.closure_checks
    : Array.isArray(research?.closure_checks)
      ? research.closure_checks
      : [];
  return source
    .map((item) => ({
      id: cleanDisplayText(item?.id || "", ""),
      label: cleanDisplayText(item?.label || "", zhLabel("检查项", "Check")),
      status: normalizeText(item?.status || "attention") === "pass" ? "pass" : "attention",
      detail: cleanDisplayText(item?.detail || "", ""),
    }))
    .filter((item) => item.label || item.detail);
}

function renderResearchProgressPrompts(research, compact = false) {
  const prompts = paperWorkflowProgressPrompts(research);
  if (!prompts.length) return "";
  const visiblePrompts = compact ? prompts.slice(0, 2) : prompts;
  return `
    <div class="research-progress-prompts">
      <div class="research-progress-prompts-head">
        <span class="research-progress-prompts-title">${escapeHtml(zhLabel("研究进度提示", "Research progress prompts"))}</span>
        <span class="research-progress-prompts-count">${escapeHtml(`${visiblePrompts.length}/${prompts.length}`)}</span>
      </div>
      <div class="research-progress-prompts-list">
        ${visiblePrompts.map((item) => `
          <div class="research-progress-prompt">
            <div class="research-progress-prompt-head">
              <span class="research-progress-prompt-stage">${escapeHtml(item.stageLabel || item.stageId || zhLabel("阶段", "Stage"))}</span>
            </div>
            <div class="research-progress-prompt-message">${escapeHtml(item.message)}</div>
            ${item.evidenceContract.length ? `
              <div class="research-progress-prompt-contract">
                ${item.evidenceContract.slice(0, compact ? 3 : 6).map((contract) => `<span>${escapeHtml(contract)}</span>`).join("")}
              </div>
            ` : ""}
          </div>
        `).join("")}
      </div>
    </div>
  `;
}

function renderResearchClosureChecks(research, compact = false) {
  const checks = paperWorkflowClosureChecks(research);
  if (!checks.length) return "";
  const visibleChecks = compact ? checks.slice(0, 4) : checks;
  const passed = checks.filter((item) => item.status === "pass").length;
  return `
    <div class="research-closure-checks">
      <div class="research-closure-checks-head">
        <span class="research-closure-checks-title">${escapeHtml(zhLabel("研究闭环审查", "Research closure audit"))}</span>
        <span class="research-closure-checks-count">${escapeHtml(`${passed}/${checks.length}`)}</span>
      </div>
      <div class="research-closure-checks-list">
        ${visibleChecks.map((item) => `
          <div class="research-closure-check is-${escapeHtml(item.status)}">
            <div class="research-closure-check-head">
              <span class="paper-workflow-pill ${item.status === "pass" ? "is-ready" : "is-attention"}">${escapeHtml(item.status === "pass" ? zhLabel("通过", "Pass") : zhLabel("需处理", "Needs work"))}</span>
              <span class="research-closure-check-title">${escapeHtml(item.label)}</span>
            </div>
            ${item.detail ? `<div class="research-closure-check-detail">${escapeHtml(item.detail)}</div>` : ""}
          </div>
        `).join("")}
      </div>
    </div>
  `;
}

function paperWorkflowSectionDiffItems(paperWorkflow) {
  return Array.isArray(paperWorkflow?.section_diff_preview)
    ? paperWorkflow.section_diff_preview
    : [];
}

function paperWorkflowManuscriptDiffItems(paperWorkflow) {
  return Array.isArray(paperWorkflow?.manuscript_diff_preview)
    ? paperWorkflow.manuscript_diff_preview
    : [];
}

function paperWorkflowManuscriptDiffMap(paperWorkflow) {
  const index = new Map();
  paperWorkflowManuscriptDiffItems(paperWorkflow).forEach((item) => {
    const sectionId = cleanDisplayText(item?.section_id || "", "");
    if (sectionId) {
      index.set(normalizeText(sectionId), item);
    }
  });
  return index;
}

function paperWorkflowSectionDiffMap(paperWorkflow) {
  const index = new Map();
  paperWorkflowSectionDiffItems(paperWorkflow).forEach((item) => {
    const sectionId = cleanDisplayText(item?.section_id || "", "");
    if (sectionId) {
      index.set(normalizeText(sectionId), item);
    }
  });
  return index;
}

function summarizeClaimAnchor(claimAnchor) {
  const claimText = cleanDisplayText(claimAnchor?.claim_text || "", "");
  const evidenceRefs = Array.isArray(claimAnchor?.evidence_refs)
    ? claimAnchor.evidence_refs
    : [];
  const requiredSources = evidenceRefs
    .filter((entry) => entry?.required === true)
    .map((entry) => cleanDisplayText(entry?.source_key || "", ""))
    .filter(Boolean);
  if (claimText && requiredSources.length) {
    return `${claimText} [${requiredSources.join(" / ")}]`;
  }
  return claimText || requiredSources.join(" / ");
}

function paperClaimSemanticStatusLabel(status) {
  const normalized = normalizeText(status || "");
  if (normalized === "strong") return zhLabel("强", "Strong");
  if (normalized === "supported") return zhLabel("已落地", "Grounded");
  if (normalized === "contradicted") return zhLabel("矛盾", "Contradicted");
  if (normalized === "weak") return zhLabel("弱", "Weak");
  if (normalized === "missing_section_text") return zhLabel("缺少章节文本", "No section text");
  return zhLabel("缺少语义支持", "Missing semantic support");
}

function paperClaimSemanticRelationLabel(relation) {
  const normalized = normalizeText(relation || "");
  if (normalized === "entailed") return zhLabel("蕴含", "Entailed");
  if (normalized === "supported") return zhLabel("支持", "Supported");
  if (normalized === "contradicted") return zhLabel("矛盾", "Contradicted");
  if (normalized === "mixed") return zhLabel("混合", "Mixed");
  if (normalized === "missing_section_text") return zhLabel("缺少章节", "Missing section");
  return zhLabel("不支持", "Unsupported");
}

function renderPaperClaimSentenceAlignments(claim, limit = 2) {
  const alignments = Array.isArray(claim?.claim_sentence_alignments) ? claim.claim_sentence_alignments.slice(0, limit) : [];
  if (!alignments.length) return "";
  return `
    <div class="paper-claim-sentence-list">
      ${alignments.map((alignment) => `
        <div class="paper-claim-sentence-card">
          <div class="paper-claim-sentence-head">
            <span>${escapeHtml(paperClaimSemanticRelationLabel(alignment?.relation || ""))}</span>
            <span>${escapeHtml(zhLabel(`分数 ${Number(alignment?.support_score || 0) || 0}`, `score ${Number(alignment?.support_score || 0) || 0}`))}</span>
          </div>
          <div class="paper-claim-sentence-text">${escapeHtml(cleanDisplayText(alignment?.claim_unit || "", ""))}</div>
          <div class="paper-claim-sentence-text is-grounded">${escapeHtml(cleanDisplayText(alignment?.grounded_sentence || "", zhLabel("该片段内没有捕获到更细粒度的落地句子。", "No finer-grained grounded sentence was captured inside the span.")))}</div>
        </div>
      `).join("")}
    </div>
  `;
}

function paperWorkspaceSectionKey(sectionId, title) {
  return normalizeText(sectionId || title || "");
}

function paperWorkflowWorkspaceSections(paperWorkflow) {
  const sections = new Map();
  const upsert = (sectionId, title, extra = {}) => {
    const cleanId = cleanDisplayText(sectionId || "", "");
    const cleanTitle = cleanDisplayText(title || "", "");
    const key = paperWorkspaceSectionKey(cleanId, cleanTitle);
    if (!key) return;
    const current = sections.get(key) || {
      key,
      sectionId: cleanId,
      title: cleanTitle || cleanId,
      claimCount: 0,
      feedbackCount: 0,
      changed: false,
    };
    sections.set(key, {
      ...current,
      sectionId: current.sectionId || cleanId,
      title: current.title || cleanTitle || cleanId,
      claimCount: Math.max(Number(current.claimCount || 0), Number(extra.claimCount || 0)),
      feedbackCount: Math.max(Number(current.feedbackCount || 0), Number(extra.feedbackCount || 0)),
      changed: current.changed || extra.changed === true,
    });
  };

  paperWorkflowClaimGateChecks(paperWorkflow).forEach((claim) => {
    const sectionId = cleanDisplayText(claim?.section_id || "", "");
    const title = cleanDisplayText(claim?.section_title || "", "");
    const key = paperWorkspaceSectionKey(sectionId, title);
    const current = sections.get(key);
    upsert(sectionId, title, { claimCount: Number(current?.claimCount || 0) + 1 });
  });

  paperWorkflowSectionDiffItems(paperWorkflow).forEach((item) => {
    upsert(item?.section_id, item?.title, { changed: item?.changed === true });
  });
  paperWorkflowManuscriptDiffItems(paperWorkflow).forEach((item) => {
    upsert(item?.section_id, item?.title, { changed: item?.changed === true });
  });
  paperWorkflowFlowItems(paperWorkflow).forEach((item) => {
    cleanDisplayList(item?.targetSections || []).forEach((sectionId) => {
      const key = paperWorkspaceSectionKey(sectionId, sectionId);
      const current = sections.get(key);
      upsert(sectionId, sectionId, { feedbackCount: Number(current?.feedbackCount || 0) + 1 });
    });
    (Array.isArray(item?.diffCards) ? item.diffCards : []).forEach((diffItem) => {
      const key = paperWorkspaceSectionKey(diffItem?.sectionId, diffItem?.title);
      const current = sections.get(key);
      upsert(diffItem?.sectionId, diffItem?.title, {
        feedbackCount: Number(current?.feedbackCount || 0) + 1,
        changed: diffItem?.changed === true,
      });
    });
  });

  return Array.from(sections.values());
}

function resolvePaperWorkspaceSelection(paperWorkflow) {
  const sections = paperWorkflowWorkspaceSections(paperWorkflow);
  const selectedKey = paperWorkspaceSectionKey(paperWorkspaceState.sectionId, paperWorkspaceState.label);
  const activeSection = sections.find((item) => item.key === selectedKey) || sections[0] || null;
  if (activeSection) {
    paperWorkspaceState.sectionId = activeSection.sectionId || "";
    paperWorkspaceState.label = activeSection.title || activeSection.sectionId || "";
  }
  return { sections, activeSection };
}

function paperWorkflowFeedbackItemsForSection(paperWorkflow, sectionId) {
  const key = paperWorkspaceSectionKey(sectionId, sectionId);
  return paperWorkflowFlowItems(paperWorkflow).filter((item) => {
    const targets = Array.isArray(item?.targetSections) ? item.targetSections : [];
    if (targets.some((entry) => paperWorkspaceSectionKey(entry, entry) === key)) {
      return true;
    }
    const diffCards = Array.isArray(item?.diffCards) ? item.diffCards : [];
    return diffCards.some((diffItem) => paperWorkspaceSectionKey(diffItem?.sectionId, diffItem?.title) === key);
  });
}

function renderPaperWorkspaceViewer(paperWorkflow) {
  const { sections, activeSection } = resolvePaperWorkspaceSelection(paperWorkflow);
  if (!activeSection) {
    return `
      <div class="paper-workspace-viewer">
        <div class="paper-workspace-viewer-empty">${escapeHtml(zhLabel("当前 workflow 暂无可同步的分节查看器。", "No synchronized section viewer is available for the current workflow yet."))}</div>
      </div>
    `;
  }

  const sectionKey = activeSection.key;
  const claimChecks = paperWorkflowClaimGateChecks(paperWorkflow).filter((claim) => {
    return paperWorkspaceSectionKey(claim?.section_id, claim?.section_title) === sectionKey;
  });
  const sectionDiff = paperWorkflowSectionDiffMap(paperWorkflow).get(sectionKey) || null;
  const manuscriptDiff = paperWorkflowManuscriptDiffMap(paperWorkflow).get(sectionKey) || null;
  const feedbackItems = paperWorkflowFeedbackItemsForSection(paperWorkflow, activeSection.sectionId);
  const currentExcerpt = cleanDisplayText(
    manuscriptDiff?.after?.markdown_excerpt
      || manuscriptDiff?.afterText
      || sectionDiff?.after?.markdown_excerpt
      || sectionDiff?.afterText
      || claimChecks[0]?.manuscript_excerpt
      || claimChecks[0]?.grounded_section_span_excerpt
      || "",
    "",
  );
  const previousExcerpt = cleanDisplayText(
    manuscriptDiff?.before?.markdown_excerpt
      || manuscriptDiff?.beforeText
      || sectionDiff?.before?.markdown_excerpt
      || sectionDiff?.beforeText
      || "",
    "",
  );
  const changedFields = cleanDisplayList(
    manuscriptDiff?.changed_fields || sectionDiff?.changed_fields || [],
  );
  const reverificationScope = cleanDisplayList(
    manuscriptDiff?.after?.reverification_scope
      || sectionDiff?.after?.reverification_scope
      || sectionDiff?.before?.reverification_scope
      || [],
  );
  const beforeClaims = Array.isArray(manuscriptDiff?.before?.claim_anchors)
    ? manuscriptDiff.before.claim_anchors
    : Array.isArray(sectionDiff?.before?.claim_anchors)
      ? sectionDiff.before.claim_anchors
      : [];
  const afterClaims = Array.isArray(manuscriptDiff?.after?.claim_anchors)
    ? manuscriptDiff.after.claim_anchors
    : Array.isArray(sectionDiff?.after?.claim_anchors)
      ? sectionDiff.after.claim_anchors
      : [];

  return `
    <div class="paper-workspace-viewer">
      <div class="paper-workspace-viewer-head">
        <div>
          <div class="paper-workspace-band-title">${escapeHtml(activeSection.title || activeSection.sectionId || zhLabel("章节", "Section"))}</div>
          <div class="paper-workspace-meta">${escapeHtml([
            activeSection.sectionId,
            activeSection.claimCount ? zhLabel(`${activeSection.claimCount} 条 claim`, `${activeSection.claimCount} claims`) : "",
            activeSection.feedbackCount ? zhLabel(`${activeSection.feedbackCount} 条反馈`, `${activeSection.feedbackCount} feedback items`) : "",
            activeSection.changed ? zhLabel("已改写", "Rewritten") : zhLabel("未改动", "Unchanged"),
          ].filter(Boolean).join(" / "))}</div>
        </div>
        ${reverificationScope.length ? `<div class="paper-workspace-meta">${escapeHtml(reverificationScope.join(" / "))}</div>` : ""}
      </div>
      <div class="paper-workspace-section-strip">
        ${sections.map((section) => `
          <button
            type="button"
            class="paper-workspace-section-pill${section.key === sectionKey ? " is-active" : ""}"
            data-paper-workspace-section="${escapeHtml(section.sectionId || section.title || "")}"
            data-paper-workspace-title="${escapeHtml(section.title || section.sectionId || "")}"
          >
            <strong>${escapeHtml(section.title || section.sectionId || zhLabel("章节", "Section"))}</strong>
            <span>${escapeHtml([
              section.claimCount ? String(section.claimCount) : "",
              section.feedbackCount ? `R${section.feedbackCount}` : "",
              section.changed ? zhLabel("Diff", "Diff") : "",
            ].filter(Boolean).join(" / "))}</span>
          </button>
        `).join("")}
      </div>
      <div class="paper-workspace-viewer-grid">
        <section class="paper-workspace-viewer-pane">
          <div class="paper-workspace-viewer-label">${escapeHtml(zhLabel("当前稿件", "Current manuscript"))}</div>
          <div class="paper-workspace-viewer-body markdown-body">${currentExcerpt ? renderMarkdown(currentExcerpt) : escapeHtml(zhLabel("该章节暂时没有可展示的当前稿件摘录。", "No current manuscript excerpt is available for this section yet."))}</div>
          ${afterClaims.length ? `
            <div class="paper-workspace-viewer-chip-list">
              ${afterClaims.slice(0, 3).map((claimAnchor) => `<div class="paper-workspace-viewer-chip">${escapeHtml(summarizeClaimAnchor(claimAnchor))}</div>`).join("")}
            </div>
          ` : ""}
        </section>
        <section class="paper-workspace-viewer-pane">
          <div class="paper-workspace-viewer-label">${escapeHtml(zhLabel("Review / Rebuttal", "Review / Rebuttal"))}</div>
          <div class="paper-workspace-viewer-stack">
            ${feedbackItems.length ? feedbackItems.map((item) => `
              <div class="paper-workspace-review-card">
                <div class="paper-workspace-review-head">
                  <strong>${escapeHtml(item.reviewer || zhLabel("审稿人", "Reviewer"))}</strong>
                  <span>${escapeHtml(item.closureStatus || item.closureState || zhLabel("未关闭", "Open"))}</span>
                </div>
                <div class="paper-workspace-review-text">${escapeHtml(item.comment || zhLabel("暂无评论内容", "No comment text"))}</div>
                ${item.executionNote ? `<div class="paper-workspace-review-meta">${escapeHtml(item.executionNote)}</div>` : ""}
                ${item.closureFollowup ? `<div class="paper-workspace-review-meta">${escapeHtml(item.closureFollowup)}</div>` : ""}
              </div>
            `).join("") : `<div class="paper-workspace-viewer-empty">${escapeHtml(zhLabel("该章节还没有关联的 reviewer closure 记录。", "No reviewer-closure records are linked to this section."))}</div>`}
          </div>
        </section>
        <section class="paper-workspace-viewer-pane">
          <div class="paper-workspace-viewer-label">${escapeHtml(zhLabel("Claim Gate", "Claim gate"))}</div>
          <div class="paper-workspace-viewer-stack">
            ${claimChecks.length ? claimChecks.map((claim) => `
              <div class="paper-workspace-claim-card ${normalizeText(claim?.status || "") === "pass" ? "is-ready" : "is-attention"}">
                <div class="paper-workspace-review-head">
                  <strong>${escapeHtml(cleanDisplayText(claim?.claim_id || "", zhLabel("Claim", "Claim")))}</strong>
                  <span>${escapeHtml(paperClaimSemanticRelationLabel(claim?.semantic_relation || claim?.semantic_support_status || ""))}</span>
                </div>
                <div class="paper-workspace-review-text">${escapeHtml(cleanDisplayText(claim?.claim_text || "", ""))}</div>
                ${cleanDisplayText(claim?.semantic_relation_detail || "", "") ? `<div class="paper-workspace-review-meta">${escapeHtml(cleanDisplayText(claim?.semantic_relation_detail || "", ""))}</div>` : ""}
                ${cleanDisplayText(claim?.grounded_section_span_excerpt || "", "") ? `<div class="paper-workspace-review-meta">${escapeHtml(cleanDisplayText(claim?.grounded_section_span_excerpt || "", ""))}</div>` : ""}
                ${renderPaperClaimSentenceAlignments(claim, 2)}
              </div>
            `).join("") : `<div class="paper-workspace-viewer-empty">${escapeHtml(zhLabel("该章节暂时没有 claim-level gate 记录。", "No claim-level gate records are available for this section yet."))}</div>`}
          </div>
        </section>
      </div>
      <div class="paper-workspace-diff-grid">
        <section class="paper-workspace-viewer-pane">
          <div class="paper-workspace-viewer-label">${escapeHtml(zhLabel("Before", "Before"))}</div>
          <div class="paper-workspace-viewer-body markdown-body">${previousExcerpt ? renderMarkdown(previousExcerpt) : escapeHtml(zhLabel("没有修订前摘录。", "No previous draft excerpt."))}</div>
          ${beforeClaims.length ? `
            <div class="paper-workspace-viewer-chip-list">
              ${beforeClaims.slice(0, 3).map((claimAnchor) => `<div class="paper-workspace-viewer-chip">${escapeHtml(summarizeClaimAnchor(claimAnchor))}</div>`).join("")}
            </div>
          ` : ""}
        </section>
        <section class="paper-workspace-viewer-pane">
          <div class="paper-workspace-viewer-label">${escapeHtml(zhLabel("After / Diff", "After / Diff"))}</div>
          <div class="paper-workspace-viewer-body markdown-body">${currentExcerpt ? renderMarkdown(currentExcerpt) : escapeHtml(zhLabel("没有修订后摘录。", "No revised draft excerpt."))}</div>
          ${changedFields.length ? `<div class="paper-workspace-viewer-chip-list">${changedFields.map((item) => `<div class="paper-workspace-viewer-chip">${escapeHtml(item)}</div>`).join("")}</div>` : ""}
        </section>
      </div>
    </div>
  `;
}

function paperWorkflowFlowItems(paperWorkflow) {
  const trace = Array.isArray(paperWorkflow?.reviewer_feedback_trace)
    ? paperWorkflow.reviewer_feedback_trace
    : [];
  const closureRecords = Array.isArray(paperWorkflow?.rebuttal_closure_records)
    ? paperWorkflow.rebuttal_closure_records
    : [];
  const executionSections = Array.isArray(paperWorkflow?.revision_execution_trace?.executed_sections)
    ? paperWorkflow.revision_execution_trace.executed_sections
    : [];
  const diffMap = paperWorkflowSectionDiffMap(paperWorkflow);
  const manuscriptDiffMap = paperWorkflowManuscriptDiffMap(paperWorkflow);
  return trace.map((entry, index) => {
    const feedbackIndex = Number(entry?.feedback_index ?? index) || index;
    const closure = closureRecords.find((item) => Number(item?.feedback_index ?? -1) === feedbackIndex) || null;
    const execution = executionSections.find((item) => Number(item?.feedback_index ?? -1) === feedbackIndex) || null;
    const targetSections = cleanDisplayList(entry?.target_sections || []);
    const sectionDiffs = targetSections
      .map((sectionId) => diffMap.get(normalizeText(sectionId)))
      .filter(Boolean)
      .map((diffItem) => ({
        sectionId: cleanDisplayText(diffItem?.section_id || "", ""),
        title: cleanDisplayText(diffItem?.title || "", ""),
        changed: diffItem?.changed === true,
        changedFields: cleanDisplayList(diffItem?.changed_fields || []),
        beforeText: cleanDisplayText(diffItem?.before?.markdown_excerpt || diffItem?.before?.draft_seed || "", ""),
        afterText: cleanDisplayText(diffItem?.after?.markdown_excerpt || diffItem?.after?.draft_seed || "", ""),
        beforeWordCount: Number(diffItem?.before?.word_count || 0) || 0,
        afterWordCount: Number(diffItem?.after?.word_count || 0) || 0,
        beforeDirective: cleanDisplayText(diffItem?.before?.revision_directive || "", ""),
        afterDirective: cleanDisplayText(diffItem?.after?.revision_directive || "", ""),
        reverificationScope: cleanDisplayList(
          diffItem?.after?.reverification_scope
          || diffItem?.before?.reverification_scope
          || [],
        ),
        beforeClaims: Array.isArray(diffItem?.before?.claim_anchors) ? diffItem.before.claim_anchors : [],
        afterClaims: Array.isArray(diffItem?.after?.claim_anchors) ? diffItem.after.claim_anchors : [],
      }));
    const manuscriptDiffs = targetSections
      .map((sectionId) => manuscriptDiffMap.get(normalizeText(sectionId)))
      .filter(Boolean)
      .map((diffItem) => ({
        sectionId: cleanDisplayText(diffItem?.section_id || "", ""),
        title: cleanDisplayText(diffItem?.title || "", ""),
        changed: diffItem?.changed === true,
        changedFields: cleanDisplayList(diffItem?.changed_fields || []),
        beforeText: cleanDisplayText(diffItem?.before?.markdown_excerpt || "", ""),
        afterText: cleanDisplayText(diffItem?.after?.markdown_excerpt || "", ""),
        beforeWordCount: Number(diffItem?.before?.word_count || 0) || 0,
        afterWordCount: Number(diffItem?.after?.word_count || 0) || 0,
        beforeClaims: Array.isArray(diffItem?.before?.claim_anchors) ? diffItem.before.claim_anchors : [],
        afterClaims: Array.isArray(diffItem?.after?.claim_anchors) ? diffItem.after.claim_anchors : [],
      }));
    const diffCardMap = new Map();
    sectionDiffs.forEach((diffItem) => {
      diffCardMap.set(normalizeText(diffItem.sectionId || diffItem.title || ""), { ...diffItem });
    });
    manuscriptDiffs.forEach((diffItem) => {
      const key = normalizeText(diffItem.sectionId || diffItem.title || "");
      const current = diffCardMap.get(key) || {};
      diffCardMap.set(key, {
        ...current,
        ...diffItem,
        changedFields: [...new Set([...(current.changedFields || []), ...(diffItem.changedFields || [])])],
        beforeText: diffItem.beforeText || current.beforeText || "",
        afterText: diffItem.afterText || current.afterText || "",
        beforeWordCount: diffItem.beforeWordCount || current.beforeWordCount || 0,
        afterWordCount: diffItem.afterWordCount || current.afterWordCount || 0,
        beforeClaims: diffItem.beforeClaims?.length ? diffItem.beforeClaims : (current.beforeClaims || []),
        afterClaims: diffItem.afterClaims?.length ? diffItem.afterClaims : (current.afterClaims || []),
      });
    });
    return {
      feedbackIndex,
      reviewer: cleanDisplayText(entry?.reviewer || "", zhLabel("审稿人", "Reviewer")),
      comment: cleanDisplayText(entry?.comment || "", ""),
      closureState: cleanDisplayText(entry?.closure_state || "", ""),
      targetSections,
      reverificationRequired: entry?.reverification_required === true,
      closureStatus: cleanDisplayText(closure?.response_status || "", ""),
      closureFollowup: cleanDisplayText(closure?.required_followup || "", ""),
      executionScope: cleanDisplayList(execution?.reverification_scope || []),
      executionNote: cleanDisplayText(execution?.closure_note || "", ""),
      rewriteActions: cleanDisplayList(execution?.rewrite_actions || []),
      sectionDiffs,
      manuscriptDiffs,
      diffCards: Array.from(diffCardMap.values()),
    };
  });
}

function renderPaperReadyGate(paperWorkflow, compact = false) {
  const checks = paperWorkflowGateChecks(paperWorkflow);
  if (!checks.length || compact) return "";
  const claimChecks = paperWorkflowClaimGateChecks(paperWorkflow);
  return `
    <div class="paper-ready-gate">
      <div class="paper-ready-gate-head">
        <span class="paper-ready-gate-title">${escapeHtml(zhLabel("Paper-ready 证据门禁", "Paper-ready evidence gate"))}</span>
        <span class="paper-ready-gate-count">${escapeHtml(zhLabel(`${checks.length} 项检查`, `${checks.length} checks`))}</span>
      </div>
      <div class="paper-ready-gate-list">
        ${checks.map((check) => {
          const status = normalizeText(check?.status || "") === "pass" ? "is-ready" : "is-attention";
          const label = cleanDisplayText(check?.check_id || "", zhLabel("门禁检查", "Gate check"));
          const detail = cleanDisplayText(check?.detail || "", "");
          return `
            <div class="paper-ready-gate-item ${status}">
              <div class="paper-ready-gate-item-head">
                <span class="paper-workflow-pill ${status}">${escapeHtml(normalizeText(check?.status || "") === "pass" ? zhLabel("通过", "Pass") : zhLabel("失败", "Fail"))}</span>
                <span class="paper-ready-gate-item-title">${escapeHtml(label)}</span>
              </div>
              <div class="paper-ready-gate-item-detail">${escapeHtml(detail)}</div>
            </div>
          `;
        }).join("")}
      </div>
      ${claimChecks.length ? `
        <div class="paper-claim-gate">
          <div class="paper-claim-gate-head">
            <span class="paper-claim-gate-title">${escapeHtml(zhLabel("逐条 claim 证据", "Claim-by-claim evidence"))}</span>
            <span class="paper-claim-gate-count">${escapeHtml(zhLabel(`${claimChecks.length} 条 claim`, `${claimChecks.length} claims`))}</span>
          </div>
        <div class="paper-claim-gate-list">
          ${claimChecks.map((claim) => {
              const passed = normalizeText(claim?.status || "") === "pass";
              const failureSources = cleanDisplayList(claim?.failure_sources || []);
              const failureReasons = cleanDisplayList(claim?.semantic_failure_reasons || []);
              const matchedFields = cleanDisplayList(claim?.matched_result_bundle_fields || []);
              const matchedValues = cleanDisplayList(claim?.matched_result_bundle_values || []);
              const supportLabel = paperClaimSemanticStatusLabel(claim?.semantic_support_status || "");
              const relationLabel = paperClaimSemanticRelationLabel(claim?.semantic_relation || claim?.semantic_support_status || "");
              const supportScore = Number(claim?.semantic_support_score || 0) || 0;
              const claimOverlapMatched = Number(claim?.claim_anchor_overlap?.matched || 0) || 0;
              const claimOverlapTotal = Number(claim?.claim_anchor_overlap?.total || 0) || 0;
              const evidenceOverlapMatched = Number(claim?.evidence_overlap?.matched || 0) || 0;
              const evidenceOverlapTotal = Number(claim?.evidence_overlap?.total || 0) || 0;
              const groundedRequiredCount = Number(claim?.grounded_required_source_count || 0) || 0;
              const requiredCount = Number(claim?.required_source_count || 0) || 0;
              const groundedItemCount = Number(claim?.grounded_required_item_count || 0) || 0;
              const groundedItemTargetCount = Number(claim?.required_item_grounding_target_count || 0) || 0;
              const groundedSpanExcerpt = cleanDisplayText(claim?.grounded_section_span_excerpt || "", "");
              const manuscriptExcerpt = cleanDisplayText(claim?.manuscript_excerpt || "", "");
              return `
                <div class="paper-claim-gate-item ${passed ? "is-ready" : "is-attention"}">
                  <div class="paper-ready-gate-item-head">
                    <span class="paper-workflow-pill ${passed ? "is-ready" : "is-attention"}">${escapeHtml(passed ? zhLabel("通过", "Pass") : zhLabel("缺少证据", "Missing evidence"))}</span>
                    <span class="paper-ready-gate-item-title">${escapeHtml(cleanDisplayText(claim?.claim_id || "", zhLabel("Claim", "Claim")))}</span>
                  </div>
                  <div class="paper-claim-gate-meta-row">
                    <span>${escapeHtml(cleanDisplayText(claim?.section_title || "", zhLabel("章节缺失", "Section missing")))}</span>
                    <span>${escapeHtml(`${relationLabel} / ${supportLabel} / ${zhLabel("分数", "score")} ${supportScore}`)}</span>
                  </div>
                  <div class="paper-ready-gate-item-detail">${escapeHtml(cleanDisplayText(claim?.claim_text || "", ""))}</div>
                  ${cleanDisplayText(claim?.semantic_relation_detail || "", "") ? `<div class="paper-claim-gate-support">${escapeHtml(cleanDisplayText(claim?.semantic_relation_detail || "", ""))}</div>` : ""}
                  <div class="paper-claim-gate-meta-row">
                    <span>${escapeHtml(zhLabel(`Claim 重叠 ${claimOverlapMatched}/${claimOverlapTotal}`, `Claim overlap ${claimOverlapMatched}/${claimOverlapTotal}`))}</span>
                    <span>${escapeHtml(zhLabel(`证据重叠 ${evidenceOverlapMatched}/${evidenceOverlapTotal}`, `Evidence overlap ${evidenceOverlapMatched}/${evidenceOverlapTotal}`))}</span>
                  </div>
                  ${requiredCount || groundedItemTargetCount ? `
                    <div class="paper-claim-gate-meta-row">
                      <span>${escapeHtml(zhLabel(`同 span 来源 ${groundedRequiredCount}/${requiredCount}`, `Same-span sources ${groundedRequiredCount}/${requiredCount}`))}</span>
                      <span>${escapeHtml(zhLabel(`同 span 证据项 ${groundedItemCount}/${groundedItemTargetCount}`, `Same-span evidence items ${groundedItemCount}/${groundedItemTargetCount}`))}</span>
                    </div>
                  ` : ""}
                  ${matchedFields.length ? `<div class="paper-claim-gate-support">${escapeHtml(matchedFields.join(" / "))}</div>` : ""}
                  ${matchedValues.length ? `<div class="paper-claim-gate-support">${escapeHtml(matchedValues.join(" / "))}</div>` : ""}
                  ${groundedSpanExcerpt ? `<div class="paper-claim-gate-support">${escapeHtml(zhLabel("Grounded span", "Grounded span"))}</div><div class="paper-claim-gate-excerpt">${escapeHtml(groundedSpanExcerpt)}</div>` : ""}
                  ${renderPaperClaimSentenceAlignments(claim, 2)}
                  ${manuscriptExcerpt ? `<div class="paper-claim-gate-excerpt">${escapeHtml(manuscriptExcerpt)}</div>` : ""}
                  ${failureReasons.length ? `<div class="paper-claim-gate-reasons">${escapeHtml(failureReasons.join(" / "))}</div>` : ""}
                  ${failureSources.length ? `<div class="paper-review-flow-item-meta">${escapeHtml(failureSources.join(" / "))}</div>` : ""}
                </div>
              `;
            }).join("")}
          </div>
        </div>
      ` : ""}
    </div>
  `;
}

function renderPaperReviewFlow(paperWorkflow, compact = false) {
  const items = paperWorkflowFlowItems(paperWorkflow);
  if (!items.length || compact) return "";
  return `
    <div class="paper-review-flow">
      <div class="paper-review-flow-head">
        <span class="paper-review-flow-title">${escapeHtml(zhLabel("审查闭环流程", "Review closure flow"))}</span>
        <span class="paper-review-flow-count">${escapeHtml(zhLabel(`${items.length} 条反馈`, `${items.length} feedback items`))}</span>
      </div>
      <div class="paper-review-flow-list">
        ${items.map((item) => {
          const open = normalizeText(item.closureState) !== "resolved";
          const sections = item.targetSections.length ? item.targetSections.join(", ") : zhLabel("讨论部分", "discussion");
          const executionScope = item.executionScope.length ? item.executionScope.join(" / ") : zhLabel("无", "None");
          return `
            <article class="paper-review-flow-item ${open ? "is-open" : "is-closed"}">
              <div class="paper-review-flow-item-head">
                <span class="paper-workflow-pill ${open ? "is-attention" : "is-ready"}">${escapeHtml(open ? zhLabel("未关闭", "Open") : zhLabel("已关闭", "Closed"))}</span>
                <span class="paper-review-flow-item-title">${escapeHtml(`${item.reviewer} #${item.feedbackIndex + 1}`)}</span>
              </div>
              <div class="paper-review-flow-item-comment">${escapeHtml(item.comment || zhLabel("暂无评论内容", "No comment text"))}</div>
              <div class="paper-review-flow-item-grid">
                <div class="paper-review-flow-item-row">
                  <span class="paper-review-flow-label">${escapeHtml(zhLabel("目标章节", "Target sections"))}</span>
                  <span class="paper-review-flow-value">${escapeHtml(sections)}</span>
                </div>
                <div class="paper-review-flow-item-row">
                  <span class="paper-review-flow-label">${escapeHtml(zhLabel("复核范围", "Reverify"))}</span>
                  <span class="paper-review-flow-value">${escapeHtml(executionScope)}</span>
                </div>
                <div class="paper-review-flow-item-row">
                  <span class="paper-review-flow-label">${escapeHtml(zhLabel("闭环状态", "Closure"))}</span>
                  <span class="paper-review-flow-value">${escapeHtml(item.closureStatus || item.closureState || "")}</span>
                </div>
              </div>
              ${item.executionNote ? `<div class="paper-review-flow-item-meta">${escapeHtml(item.executionNote)}</div>` : ""}
              ${item.closureFollowup ? `<div class="paper-review-flow-item-meta">${escapeHtml(item.closureFollowup)}</div>` : ""}
            </article>
          `;
        }).join("")}
      </div>
    </div>
  `;
}

function renderPaperWorkflowPanel(research, options = {}) {
  const compact = options.compact === true;
  const paperWorkflow = research?.paper_workflow || null;
  if (!paperWorkflow) return "";
  const artifacts = compact ? paperWorkflowArtifacts(research).slice(0, 3) : paperWorkflowArtifacts(research);
  const summary = cleanDisplayText(paperWorkflow?.summary || "", t("paperWorkflowSummaryFallback")) || t("paperWorkflowSummaryFallback");
  const primaryPath = paperWorkflowPrimaryPath(research);
  const lifecycleBadge = paperWorkflowLifecycleBadge(research);
  return `
    <section class="paper-workflow-panel${compact ? " is-compact" : ""}">
      <div class="paper-workflow-head">
        <div>
          <div class="paper-workflow-title">${escapeHtml(t("paperWorkflowTitle"))}</div>
          <div class="paper-workflow-summary">${escapeHtml(summary)}</div>
        </div>
        ${compact ? "" : `<button class="paper-workflow-run" type="button" data-paper-workflow-run="true">${escapeHtml(t("paperWorkflowRun"))}</button>`}
      </div>
      <div class="paper-workflow-status-row">
        <span class="paper-workflow-pill ${escapeHtml(lifecycleBadge.className)}">${escapeHtml(lifecycleBadge.label)}</span>
        ${paperWorkflow?.pdf_compile_status ? `<span class="paper-workflow-pill ${escapeHtml(paperPdfStatusClass(paperWorkflow.pdf_compile_status))}">${escapeHtml(paperPdfStatusLabel(paperWorkflow.pdf_compile_status))}</span>` : ""}
      </div>
      ${primaryPath ? `<button class="paper-workflow-primary" type="button" data-open-workspace-file="${escapeHtml(primaryPath)}">${escapeHtml(primaryPath)}</button>` : ""}
      ${compact ? "" : renderPaperWorkspace(paperWorkflow)}
      ${renderResearchProgressPrompts(research, compact)}
      ${renderResearchClosureChecks(research, compact)}
      ${renderPaperRevisionQueue(paperWorkflow, compact)}
      ${renderPaperReadyGate(paperWorkflow, compact)}
      ${renderPaperReviewFlow(paperWorkflow, compact)}
      ${artifacts.length ? `
        <div class="paper-workflow-artifact-list">
          ${artifacts.map((artifact) => `
            <button class="paper-workflow-artifact" type="button" data-open-workspace-file="${escapeHtml(cleanDisplayText(artifact.path || ""))}">
              <span class="paper-workflow-artifact-main">
                <strong>${escapeHtml(cleanDisplayText(artifact.label || "", t("paperWorkflowArtifacts")))}</strong>
                <span>${escapeHtml(cleanDisplayText(artifact.path || ""))}</span>
              </span>
              <span class="paper-workflow-artifact-kind">${escapeHtml(paperWorkflowArtifactKindLabel(artifact.kind || ""))}</span>
            </button>
          `).join("")}
        </div>
      ` : `<div class="paper-workflow-empty">${escapeHtml(t("paperWorkflowEmpty"))}</div>`}
    </section>
  `;
}

function renderPaperWorkspace(paperWorkflow, context = {}) {
  if (!paperWorkflow) return "";
  const viewer = renderPaperWorkspaceViewer(paperWorkflow);
  const links = [
    cleanDisplayText(paperWorkflow?.paper_pdf_path || "", ""),
    cleanDisplayText(paperWorkflow?.paper_latex_path || "", ""),
    cleanDisplayText(paperWorkflow?.paper_markdown_path || "", ""),
    cleanDisplayText(paperWorkflow?.review_response_path || "", ""),
  ].filter(Boolean);
  return `
    <div class="paper-workspace-shell">
      ${viewer}
      ${links.length ? `
        <div class="paper-workspace-band">
          <div class="paper-workspace-band-title">${escapeHtml(zhLabel("相关文件", "Linked files"))}</div>
          <div class="paper-workspace-link-row">
            ${links.map((path) => `
              <button class="paper-workspace-link" type="button" data-open-workspace-file="${escapeHtml(path)}">
                <strong>${escapeHtml(basename(path))}</strong>
                <span>${escapeHtml(path)}</span>
              </button>
            `).join("")}
          </div>
        </div>
      ` : ""}
    </div>
  `;
}

async function runPaperWorkflow(options = {}) {
  const silent = options.silent === true;
  const autoTriggered = options.autoTriggered === true;
  const sessionId = await ensureSessionReady();
  const researchTopic = cleanDisplayText(bootstrapData?.research?.topic || "", "");
  const normalizedResearchTopic = normalizeText(researchTopic);
  const fallbackTopic = [...visibleConversationMessages(bootstrapData?.messages || [])]
    .reverse()
    .find((message) => message?.kind === "message" && message?.role === "user" && cleanDisplayText(message?.content || "", ""))
    ?.content || "";
  const topic = normalizedResearchTopic === normalizeText(t("researchTemplateTopic"))
      || normalizedResearchTopic === normalizeText("general scientific inquiry")
      || !researchTopic
    ? cleanDisplayText(fallbackTopic, "")
    : researchTopic;
  if (!sessionId || !topic) {
    showToast(t("toastSendFailed"));
    return;
  }
  if (paperWorkflowPendingSessions.has(sessionId)) {
    return;
  }
  paperWorkflowPendingSessions.add(sessionId);
  try {
    if (!silent) {
      showToast(t("paperWorkflowRunning"));
    }
    const response = await hostClient.research.paperWorkflow({
      topic,
      session_id: sessionId,
    });
    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(errorText || `paper workflow failed: ${response.status}`);
    }
    const payload = await response.json();
    const nextData = payload?.data || payload || {};
    bootstrapData = {
      ...(bootstrapData || {}),
      current_session_id: nextData.session_id || bootstrapData?.current_session_id || sessionId,
      research: nextData.research || bootstrapData?.research || null,
    };
    if (!autoTriggered) {
      paperWorkflowAutoTriggeredSessions.add(sessionId);
    }
    renderResearch(bootstrapData?.research || null);
    bindTurnInteractionHandlers(researchPanel || document);
    bindTurnInteractionHandlers(researchDetailPanel || document);
    if (!silent) {
      showToast(t("toastPaperWorkflowDone"));
    }
  } finally {
    paperWorkflowPendingSessions.delete(sessionId);
  }
}

function renderResearch(research) {
  if (!researchPanel) return;
  syncReviewerFeedbackDraftFromResearch(research);
  const topic =
    research?.topic && research.topic !== "General scientific inquiry"
      ? cleanDisplayText(research.topic, t("researchTemplateTopic"))
      : t("researchTemplateTopic");
  const graphMarkup = researchGraphMarkup(research);
  const nodes = graphMarkup.nodes;
  const currentNode = graphMarkup.currentNode;
  const reviewItems = cleanDisplayList(research?.review || []);
  const resumePoints = cleanDisplayList(research?.resume_points || []);
  const runtimeSubagents = Array.isArray(research?.runtime?.subagents) ? research.runtime.subagents : [];
  const runtimeVerifier = research?.runtime?.verifier || null;
  const runtimeCheckpoints = cleanDisplayList(research?.runtime?.checkpoints || []);
  const runtimeTimeline = Array.isArray(research?.runtime?.timeline) ? research.runtime.timeline : [];
  const reviewerFeedbackMarkup = renderReviewerFeedbackPanel(research?.reviewer_feedback || null, {
    allowForm: true,
  });
  const paperWorkflowMarkup = renderPaperWorkflowPanel(research, { compact: false });
  const resolvedResearchState = resolveResearchOverallState(research);
  const stateLabel = researchStateLabel(resolvedResearchState);
  const rationale = cleanDisplayText(
    research?.rationale || `${t("researchAssessmentGood")} ${t("researchAssessmentHuman")}`,
    `${t("researchAssessmentGood")} ${t("researchAssessmentHuman")}`,
  );
  const resourceSummary = cleanDisplayText(research?.resource_summary || "");
  const blocker = cleanDisplayText(research?.blocker || "");
  const recoveryHint = cleanDisplayText(research?.recovery_hint || "");

  researchPanel.innerHTML = `
    <div class="research-summary">
      <div class="research-topic">${escapeHtml(topic)}</div>
      <div class="research-meta">${escapeHtml(t("researchStatus"))}: ${escapeHtml(
        currentNode?.label || research?.phase || t("researchStatusTemplate"),
      )} / ${research?.phase_index || 0}/${research?.phase_total || nodes.length || 1}</div>
      <div class="research-meta">${escapeHtml(currentLanguage === "zh" ? "流程状态" : "Workflow state")}: ${escapeHtml(stateLabel)}</div>
      <div class="research-meta">${escapeHtml(t("researchNext"))}: ${escapeHtml(
        cleanDisplayText(research?.next_phase || "", t("researchStatusTemplate")) || t("researchStatusTemplate"),
      )}</div>
      <div class="research-meta">${escapeHtml(t("researchAssessment"))}: ${escapeHtml(rationale)}</div>
      ${resourceSummary ? `<div class="research-meta">${escapeHtml(zhLabel("资源", "Resources"))}: ${escapeHtml(resourceSummary)}</div>` : ""}
      ${blocker ? `<div class="research-review-item">${escapeHtml(blocker)}</div>` : ""}
      ${recoveryHint ? `<div class="research-review-item">${escapeHtml(recoveryHint)}</div>` : ""}
    </div>
    ${graphMarkup.html}
    ${
      resumePoints.length
        ? `<div class="research-review-list">${resumePoints
            .map((item) => `<div class="research-review-item">${escapeHtml(item)}</div>`)
            .join("")}</div>`
        : ""
    }
    ${renderResearchRuntimeSubagents(runtimeSubagents, { limit: 4, outputLimit: 180, evidenceLimit: 3 })}
    ${renderResearchRuntimeVerifier(runtimeVerifier, { checkLimit: 4, issueLimit: 3 })}
    ${paperWorkflowMarkup}
    ${reviewerFeedbackMarkup}
    ${renderRuntimeTimeline(runtimeTimeline, { limit: 10, title: currentLanguage === "zh" ? "研究时间线" : "Research timeline" })}
    ${
      runtimeCheckpoints.length
        ? `<div class="research-review-list">${runtimeCheckpoints
            .slice(-3)
            .map((item) => `<div class="research-review-item">${escapeHtml(item)}</div>`)
            .join("")}</div>`
        : ""
    }
    <div class="research-review-list">
      ${reviewItems.map((item) => `<div class="research-review-item">${escapeHtml(item)}</div>`).join("")}
    </div>
    <div class="research-footer">
      <span class="research-pill ${researchSecurityClass(research?.security_level)}">${escapeHtml(
        `${t("researchSecurity")}: ${research?.security_level || "Public"}`,
      )}</span>
      ${research?.competition_mode ? `<span class="research-pill">${escapeHtml(t("researchCompetition"))}</span>` : ""}
      ${research?.waiting_approval ? `<span class="research-pill is-confidential">${escapeHtml(t("researchWaiting"))}</span>` : ""}
    </div>
    <div class="research-meta">${escapeHtml(t("researchWorkspace"))}: ${escapeHtml(
      research?.workspace || bootstrapData?.workspace_root || "",
    )}</div>
  `;
  bindTurnInteractionHandlers(researchPanel);
  renderResearchFloatingBoard(research, graphMarkup);
  renderResearchDetailPanel(research, graphMarkup);
}

function syncResearchFloatingBoardPositionFromButton() {
  if (!researchFloatingBoard || !researchFloatingReopen) return;
  if (researchFloatingBoardPosition) {
    researchFloatingBoard.style.left = `${researchFloatingBoardPosition.left}px`;
    researchFloatingBoard.style.top = `${researchFloatingBoardPosition.top}px`;
    researchFloatingBoard.style.right = "auto";
    return;
  }
  const buttonRect = researchFloatingReopen.getBoundingClientRect();
  const boardWidth = Math.min(348, Math.max(248, window.innerWidth - 132));
  const nextLeft = Math.max(8, Math.min(window.innerWidth - boardWidth - 8, buttonRect.right - boardWidth));
  const nextTop = Math.max(8, Math.min(window.innerHeight - 220, buttonRect.top - 12));
  researchFloatingBoard.style.left = `${nextLeft}px`;
  researchFloatingBoard.style.top = `${nextTop}px`;
  researchFloatingBoard.style.right = "auto";
}

function renderResearchFloatingBoard(research, graphMarkup = researchGraphMarkup(research, { minHeight: 320 })) {
  if (!researchFloatingBoard || !researchFloatingBody) return;
  const shouldShow = currentWorkspaceMode === "research" && hasResearchStartedForCurrentSession() && !researchFloatingDismissed;
  researchFloatingBoard.hidden = !shouldShow;
  if (researchFloatingReopen) {
    researchFloatingReopen.hidden =
      currentWorkspaceMode !== "research" || !hasResearchStartedForCurrentSession() || shouldShow || researchDetailOpen;
  }
  if (!shouldShow) return;
  syncResearchFloatingBoardPositionFromButton();
  const topic = cleanDisplayText(research?.topic || "", t("researchTemplateTopic")) || t("researchTemplateTopic");
  const reviewItems = cleanDisplayList((research?.review || []).slice(0, 3));
  const resumePoints = cleanDisplayList((research?.resume_points || []).slice(0, 2));
  const runtimeSubagents = Array.isArray(research?.runtime?.subagents) ? research.runtime.subagents : [];
  const runtimeVerifier = research?.runtime?.verifier || null;
  const runtimeTimeline = Array.isArray(research?.runtime?.timeline) ? research.runtime.timeline : [];
  const paperWorkflowMarkup = renderPaperWorkflowPanel(research, {
    compact: true,
  });
  const reviewerFeedbackMarkup = renderReviewerFeedbackPanel(research?.reviewer_feedback || null, {
    compact: true,
    allowForm: false,
  });
  const blocker = cleanDisplayText(research?.blocker || "");
  researchFloatingBody.innerHTML = `
    <div class="research-floating-topic">${escapeHtml(topic)}</div>
    <div class="research-floating-meta">${escapeHtml(researchWorkflowLabel(research?.workflow_kind))} / ${escapeHtml(researchStateLabel(research?.overall_state))} / ${escapeHtml(cleanDisplayText(research?.phase || ""))}</div>
    ${blocker ? `<div class="research-review-item">${escapeHtml(blocker)}</div>` : ""}
    ${graphMarkup.html}
    ${renderResearchRuntimeSubagents(runtimeSubagents, { limit: 2, outputLimit: 120, evidenceLimit: 2 })}
    ${renderResearchRuntimeVerifier(runtimeVerifier, { checkLimit: 2, issueLimit: 2 })}
    ${paperWorkflowMarkup}
    ${reviewerFeedbackMarkup}
    ${renderRuntimeTimeline(runtimeTimeline, { limit: 4, title: currentLanguage === "zh" ? "时间线" : "Timeline" })}
    <div class="research-review-list">
      ${resumePoints.map((item) => `<div class="research-review-item">${escapeHtml(item)}</div>`).join("")}
      ${reviewItems.map((item) => `<div class="research-review-item">${escapeHtml(item)}</div>`).join("")}
    </div>
  `;
  bindTurnInteractionHandlers(researchFloatingBody);
}

function renderResearchDetailPanel(research, graphMarkup = researchGraphMarkup(research, { minHeight: 420 })) {
  if (!researchDetailPanel) return;
  if (currentWorkspaceMode !== "research") {
    researchDetailPanel.innerHTML = "";
    return;
  }
  const topic = cleanDisplayText(research?.topic || "", t("researchTemplateTopic")) || t("researchTemplateTopic");
  const reviewItems = cleanDisplayList(research?.review || []);
  const resumePoints = cleanDisplayList(research?.resume_points || []);
  const runtimeSubagents = Array.isArray(research?.runtime?.subagents) ? research.runtime.subagents : [];
  const runtimeVerifier = research?.runtime?.verifier || null;
  const runtimeCheckpoints = cleanDisplayList(research?.runtime?.checkpoints || []);
  const runtimeTimeline = Array.isArray(research?.runtime?.timeline) ? research.runtime.timeline : [];
  const reviewerFeedbackMarkup = renderReviewerFeedbackPanel(research?.reviewer_feedback || null, {
    allowForm: true,
  });
  const paperWorkflowMarkup = renderPaperWorkflowPanel(research, { compact: false });
  const resourceSummary = cleanDisplayText(research?.resource_summary || "");
  const blocker = cleanDisplayText(research?.blocker || "");
  const recoveryHint = cleanDisplayText(research?.recovery_hint || "");
  researchDetailPanel.innerHTML = `
    <div class="research-detail-shell">
      <div class="research-detail-header">
        <div class="research-floating-topic">${escapeHtml(topic)}</div>
        <div class="research-floating-meta">${escapeHtml(researchWorkflowLabel(research?.workflow_kind))} / ${escapeHtml(researchStateLabel(research?.overall_state))} / ${escapeHtml(cleanDisplayText(research?.phase || ""))}</div>
      </div>
      ${resourceSummary ? `<div class="research-review-item">${escapeHtml(resourceSummary)}</div>` : ""}
      ${blocker ? `<div class="research-review-item">${escapeHtml(blocker)}</div>` : ""}
      ${recoveryHint ? `<div class="research-review-item">${escapeHtml(recoveryHint)}</div>` : ""}
      ${graphMarkup.html}
      ${
        resumePoints.length
          ? `<div class="research-review-list">${resumePoints
              .map((item) => `<div class="research-review-item">${escapeHtml(item)}</div>`)
              .join("")}</div>`
          : ""
      }
      ${renderResearchRuntimeSubagents(runtimeSubagents, { outputLimit: 220, evidenceLimit: 4 })}
      ${renderResearchRuntimeVerifier(runtimeVerifier, { checkLimit: 8, issueLimit: 5 })}
      ${paperWorkflowMarkup}
      ${reviewerFeedbackMarkup}
      ${renderRuntimeTimeline(runtimeTimeline, {
        title: currentLanguage === "zh" ? "研究时间线" : "Research timeline",
        forceOpen: true,
      })}
      ${
        runtimeCheckpoints.length
          ? `<div class="research-review-list">${runtimeCheckpoints
              .map((item) => `<div class="research-review-item">${escapeHtml(item)}</div>`)
              .join("")}</div>`
          : ""
      }
      <div class="research-review-list">
        ${reviewItems.map((item) => `<div class="research-review-item">${escapeHtml(item)}</div>`).join("")}
      </div>
    </div>
  `;
  bindTurnInteractionHandlers(researchDetailPanel);
}

function startResearchFloatingDrag(event) {
  if (!researchFloatingBoard || event.button !== 0) return;
  if (event.target instanceof HTMLElement && event.target.closest("button, a, input, textarea, select, summary")) {
    return;
  }
  const rect = researchFloatingBoard.getBoundingClientRect();
  researchFloatingDrag = {
    pointerId: event.pointerId ?? null,
    offsetX: event.clientX - rect.left,
    offsetY: event.clientY - rect.top,
  };
  researchFloatingBoard.classList.add("is-dragging");
  if (researchFloatingHead?.setPointerCapture && event.pointerId != null) {
    try {
      researchFloatingHead.setPointerCapture(event.pointerId);
    } catch (_error) {
      // Ignore pointer capture failures.
    }
  }
  event.preventDefault();
}

function moveResearchFloatingDrag(event) {
  if (!researchFloatingDrag || !researchFloatingBoard) return;
  if ("buttons" in event && (event.buttons & 1) !== 1) {
    endResearchFloatingDrag();
    return;
  }
  const boardRect = researchFloatingBoard.getBoundingClientRect();
  const boardWidth = Math.max(220, Math.round(boardRect.width || 320));
  const boardHeight = Math.max(160, Math.round(boardRect.height || 240));
  const nextLeft = Math.max(8, Math.min(window.innerWidth - boardWidth - 8, event.clientX - researchFloatingDrag.offsetX));
  const nextTop = Math.max(8, Math.min(window.innerHeight - boardHeight - 8, event.clientY - researchFloatingDrag.offsetY));
  researchFloatingBoard.style.left = `${nextLeft}px`;
  researchFloatingBoard.style.top = `${nextTop}px`;
  researchFloatingBoard.style.right = "auto";
  researchFloatingBoardPosition = { left: nextLeft, top: nextTop };
}

function endResearchFloatingDrag() {
  if (!researchFloatingDrag || !researchFloatingBoard) return;
  if (researchFloatingHead?.releasePointerCapture && researchFloatingDrag.pointerId != null) {
    try {
      researchFloatingHead.releasePointerCapture(researchFloatingDrag.pointerId);
    } catch (_error) {
      // Ignore pointer capture release failures.
    }
  }
  researchFloatingDrag = null;
  researchFloatingBoard.classList.remove("is-dragging");
}

function startResearchFloatingReopenDrag(event) {
  if (!researchFloatingReopen || event.button !== 0) return;
  researchFloatingReopenSuppressClick = false;
  const rect = researchFloatingReopen.getBoundingClientRect();
  researchFloatingReopenDrag = {
    pointerId: event.pointerId ?? null,
    startX: event.clientX,
    startY: event.clientY,
    left: rect.left,
    top: rect.top,
    moved: false,
  };
  researchFloatingReopen.classList.add("is-dragging");
  if (researchFloatingReopen.setPointerCapture && event.pointerId != null) {
    try {
      researchFloatingReopen.setPointerCapture(event.pointerId);
    } catch (_error) {
      // Ignore pointer capture failures.
    }
  }
  event.preventDefault();
}

function moveResearchFloatingReopenDrag(event) {
  if (!researchFloatingReopenDrag || !researchFloatingReopen) return;
  if ("buttons" in event && (event.buttons & 1) !== 1) {
    endResearchFloatingReopenDrag();
    return;
  }
  const deltaX = event.clientX - researchFloatingReopenDrag.startX;
  const deltaY = event.clientY - researchFloatingReopenDrag.startY;
  if (Math.abs(deltaX) > 6 || Math.abs(deltaY) > 6) {
    researchFloatingReopenDrag.moved = true;
  }
  const size = researchFloatingReopen.offsetWidth || 16;
  const nextLeft = Math.max(8, Math.min(window.innerWidth - size - 8, researchFloatingReopenDrag.left + deltaX));
  const nextTop = Math.max(8, Math.min(window.innerHeight - size - 8, researchFloatingReopenDrag.top + deltaY));
  researchFloatingReopen.style.left = `${nextLeft}px`;
  researchFloatingReopen.style.top = `${nextTop}px`;
  researchFloatingReopen.style.right = "auto";
  researchFloatingBoardPosition = null;
}

function endResearchFloatingReopenDrag() {
  if (!researchFloatingReopenDrag || !researchFloatingReopen) return;
  if (researchFloatingReopen.releasePointerCapture && researchFloatingReopenDrag.pointerId != null) {
    try {
      researchFloatingReopen.releasePointerCapture(researchFloatingReopenDrag.pointerId);
    } catch (_error) {
      // Ignore pointer capture release failures.
    }
  }
  researchFloatingReopenSuppressClick = researchFloatingReopenDrag.moved;
  researchFloatingReopen.classList.remove("is-dragging");
  researchFloatingReopenDrag = null;
}

function dismissResearchFloatingBoard() {
  researchFloatingDismissed = true;
  researchDetailOpen = false;
  dockLayout.hidden.research = true;
  saveDockLayout();
  applyDockLayout();
  researchFloatingBoardPosition = null;
  if (researchFloatingBoard) {
    researchFloatingBoard.hidden = true;
  }
  renderResearchFloatingBoard(bootstrapData?.research || null);
}

function renderProviderList(providers, primaryModel) {
  if (!providerList) return;
  providerList.innerHTML = "";
  (providers || []).forEach((providerName, index) => {
    const card = document.createElement("div");
    card.className = "provider-card";
    card.innerHTML = `
      <div>
        <div>${escapeHtml(providerName)}</div>
        <div class="provider-meta">${escapeHtml(primaryModel || "")}</div>
      </div>
      <div class="provider-state ready">${escapeHtml(index === 0 ? t("providerPrimary") : t("providerAvailable"))}</div>
    `;
    providerList.appendChild(card);
  });
}

function collectSettingsPayload() {
  const config = bootstrapData?.config || {};
  const parseLimitValue = (element, fallback) => {
    const raw = String(getSegmentedValue(element, fallback)).trim().toLowerCase();
    if (raw === "unlimited") return 0;
    const numeric = Number(raw);
    return Number.isFinite(numeric) ? numeric : Number(fallback);
  };
  const toolchains = {};
  runtimeToolchainInputs.forEach((input) => {
    const key = input.getAttribute("data-toolchain-key") || "";
    if (!key) return;
    toolchains[key] = String(input.value || "").trim();
  });
  return {
    model: String(primaryModel?.value || config.model || "").trim(),
    api_url: "https://llm-fnab949h4etu47rc.cn-beijing.maas.aliyuncs.com/compatible-mode/v1/chat/completions",
    deep_think: Boolean(deepThinkToggle?.checked ?? true),
    reasoning_effort: currentEffort,
    competition_mode: Boolean(competitionMode?.checked),
    privacy_mode: Boolean(privacyMode?.checked),
    workspace_root: String(runtimeWorkspaceRoot?.value || config.workspace_root || "").trim(),
    api_key: String(runtimeApiKey?.value || "").trim() || null,
    auto_approve_tools: Boolean(autoApproveTools?.checked),
    max_auto_approve_risk: String(getSegmentedValue(riskBoundary, config.max_auto_approve_risk || "safe")).trim().toLowerCase(),
    max_tool_calls_per_minute: parseLimitValue(maxToolCalls, "unlimited"),
    burst_limit: parseLimitValue(burstLimit, "unlimited"),
    toolchains,
  };
}

function syncSettingsFromConfig(config) {
  if (!config) return;
  if (primaryModel) primaryModel.value = config.model || "";
  if (competitionMode) competitionMode.checked = Boolean(config.competition_mode);
  if (privacyMode) privacyMode.checked = Boolean(config.privacy_mode);
  if (autoApproveTools) autoApproveTools.checked = Boolean(config.auto_approve_tools);
  setSegmentedValue(
    riskBoundary,
    normalizeChoice(config.max_auto_approve_risk || "safe", ["safe", "moderate", "low"], "safe")
  );
  setSegmentedValue(
    maxToolCalls,
    normalizeChoice(
      config.max_tool_calls_per_minute === 0 ? "unlimited" : String(config.max_tool_calls_per_minute ?? 30),
      ["10", "30", "unlimited"],
      "unlimited",
    )
  );
  setSegmentedValue(
    burstLimit,
    normalizeChoice(
      config.burst_limit === 0 ? "unlimited" : String(config.burst_limit ?? 5),
      ["1", "5", "unlimited"],
      "unlimited",
    )
  );
  if (runtimeWorkspaceRoot) runtimeWorkspaceRoot.value = config.workspace_root || "";
  if (deepThinkToggle) deepThinkToggle.checked = Boolean(config.deep_think ?? true);
  if (autoOpenActivityPanelToggle) autoOpenActivityPanelToggle.checked = autoOpenActivityPanel;
  if (runtimeApiKey) runtimeApiKey.value = config.api_key || "";
  runtimeToolchainInputs.forEach((input) => {
    const key = input.getAttribute("data-toolchain-key") || "";
    input.value = config.toolchains?.[key] || "";
  });
  currentEffort = String(config.reasoning_effort || currentEffort || "medium").toLowerCase();
  updateEffortUI();
  syncAutoApproveUI();
}

function syncWorkspaceHeader(workspaceRoot) {
  const root = workspaceRoot || bootstrapData?.config?.workspace_root || "";
  const name = basename(root);
  if (sidebarWorkspaceTitle) sidebarWorkspaceTitle.textContent = name;
  if (workspaceRootLabel) workspaceRootLabel.textContent = root;
  if (workspaceTitle) workspaceTitle.textContent = currentLanguage === "zh" ? "工作区" : "Workspace";
  if (typeof document !== "undefined") {
    document.title = `${name} / Agent Workspace`;
  }
}

function syncRiskPill(config) {
  if (!riskPill) return;
  riskPill.textContent = String(config?.max_auto_approve_risk || "safe").toLowerCase();
}

function renderBranches(branches) {
  if (!branchList) return;
  branchList.innerHTML = "";
  (branches || []).forEach((branch) => {
    const row = document.createElement("div");
    row.className = "branch-item";
    row.innerHTML = `
      <span class="branch-name">${escapeHtml(branch.name || branch.id || "main")}</span>
      <span class="branch-meta">${escapeHtml(branch.parent_id || "")}</span>
    `;
    branchList.appendChild(row);
  });
}

function formatGitStatusSummary(status) {
  if (!status) return t("gitNoStatus");
  const changedFiles = Array.isArray(status.changed_files) ? status.changed_files.length : 0;
  const parts = [];
  if (status.branch) {
    parts.push(`${t("gitBranchCurrent")}: ${status.branch}`);
  }
    if (status.upstream) {
      if ((Number(status.ahead) || 0) > 0 || (Number(status.behind) || 0) > 0) {
      parts.push(`${status.upstream} · ${template("gitAheadBehind", { ahead: Number(status.ahead) || 0, behind: Number(status.behind) || 0 })}`);
      } else {
        parts.push(status.upstream);
      }
  }
  parts.push(`${t("gitChangesSummary")}: ${changedFiles}`);
  if (status.repository_clean) parts.push(t("gitClean"));
  if (status.has_staged_changes) parts.push(t("gitStaged"));
  if (status.has_unstaged_changes) parts.push(t("gitModified"));
  if (status.has_untracked_files) parts.push(t("gitUntracked"));
  if (status.has_conflicts) parts.push(t("gitConflicted"));
  return parts.join(" · ");
}

function gitChangeTypeLabel(changeType) {
  const normalized = String(changeType || "").trim().toLowerCase();
  const labels = {
    added: currentLanguage === "zh" ? "新增" : "Added",
    modified: currentLanguage === "zh" ? "修改" : "Modified",
    deleted: currentLanguage === "zh" ? "删除" : "Deleted",
    renamed: currentLanguage === "zh" ? "重命名" : "Renamed",
    copied: currentLanguage === "zh" ? "复制" : "Copied",
    conflicted: currentLanguage === "zh" ? "冲突" : "Conflicted",
    untracked: currentLanguage === "zh" ? "未跟踪" : "Untracked",
    changed: currentLanguage === "zh" ? "变更" : "Changed",
  };
  return labels[normalized] || String(changeType || "").replace(/_/g, " ");
}

function getGitFileActions(file) {
  const actions = [];
  if (file?.staged) {
    actions.push({ action: "unstage_paths", label: t("gitUnstage") });
  } else {
    actions.push({ action: "stage_paths", label: t("gitStage") });
  }
  if (file?.unstaged || file?.untracked) {
    actions.push({ action: "discard_paths", label: t("gitDiscard"), danger: true });
  }
  return actions;
}

function renderGitOverview(git) {
  if (!gitOverviewView) return;
  const status = git?.status || null;
  const changedFiles = Array.isArray(status?.changed_files) ? status.changed_files : [];
  const commits = Array.isArray(git?.commits) ? git.commits.slice(0, 6) : [];
  gitOverviewView.innerHTML = `
    <section class="git-card-grid">
      <article class="git-card">
        <div class="git-card-label">${escapeHtml(t("gitRepository"))}</div>
        <div class="git-card-value">${escapeHtml(git?.repository_root || "")}</div>
      </article>
      <article class="git-card">
        <div class="git-card-label">${escapeHtml(t("gitBranchCurrent"))}</div>
        <div class="git-card-value">${escapeHtml(status?.branch || "main")}</div>
      </article>
      <article class="git-card">
        <div class="git-card-label">${escapeHtml(t("gitChangesSummary"))}</div>
        <div class="git-card-value">${escapeHtml(String(changedFiles.length))}</div>
      </article>
    </section>
    <section class="git-split">
      <article class="git-panel">
        <div class="git-panel-head">${escapeHtml(t("gitWorkingTree"))}</div>
        <div class="git-summary-list">
          ${
            changedFiles.length
              ? changedFiles
                  .slice(0, 12)
                  .map(
                    (file) => `
                    <div class="git-summary-row">
                      <div class="git-summary-main">
                        <span class="git-summary-path">${escapeHtml(file.path || "")}</span>
                        <span class="git-summary-type">${escapeHtml(gitChangeTypeLabel(file.change_type || file.status || ""))}</span>
                      </div>
                      <div class="git-summary-actions">
                        ${getGitFileActions(file)
                          .map(
                            (action) => `
                              <button
                                class="git-inline-action${action.danger ? " is-danger" : ""}"
                                type="button"
                                data-git-file-action="${escapeHtml(action.action)}"
                                data-git-path="${escapeHtml(file.path || "")}"
                              >
                                ${escapeHtml(action.label)}
                              </button>
                            `,
                          )
                          .join("")}
                      </div>
                    </div>
                  `,
                )
                  .join("")
              : `<div class="git-empty">${escapeHtml(t("gitNoDiff"))}</div>`
          }
        </div>
      </article>
      <article class="git-panel">
        <div class="git-panel-head">${escapeHtml(t("gitHistory"))}</div>
        <div class="git-commit-list">
          ${
            commits.length
              ? commits
                  .map(
                    (commit) => `
                      <div class="git-commit-item">
                        <div class="git-commit-main">
                          <div class="git-commit-subject">${escapeHtml(commit.message || "")}</div>
                          <div class="git-commit-meta">${escapeHtml(commit.hash || "")} / ${escapeHtml(commit.date || "")}</div>
                        </div>
                        <div class="git-commit-author">${escapeHtml(commit.author || "")}</div>
                      </div>
                    `,
                  )
                  .join("")
              : `<div class="git-empty">${escapeHtml(t("gitHistoryEmpty"))}</div>`
          }
        </div>
      </article>
    </section>
  `;
}

function renderGitChanges(git) {
  if (!gitChangesView) return;
  const status = git?.status || null;
  const changedFiles = Array.isArray(status?.changed_files) ? status.changed_files : [];
  const diffLoaded = gitDataLoadState.diff;
  const workingDiff = String(git?.working_diff || "").trim();
  const stagedDiff = String(git?.staged_diff || "").trim();
  gitChangesView.innerHTML = `
    <section class="git-panel">
      <div class="git-panel-head">${escapeHtml(t("gitWorkingTree"))}</div>
      <div class="git-summary-list git-summary-list-dense">
        ${
          changedFiles.length
            ? changedFiles
                .map(
                  (file) => `
                    <div class="git-summary-row">
                      <div class="git-summary-main">
                        <span class="git-summary-path">${escapeHtml(file.path || "")}</span>
                        <span class="git-summary-type">${escapeHtml(gitChangeTypeLabel(file.change_type || file.status || ""))}</span>
                      </div>
                      <div class="git-summary-actions">
                        ${getGitFileActions(file)
                          .map(
                            (action) => `
                              <button
                                class="git-inline-action${action.danger ? " is-danger" : ""}"
                                type="button"
                                data-git-file-action="${escapeHtml(action.action)}"
                                data-git-path="${escapeHtml(file.path || "")}"
                              >
                                ${escapeHtml(action.label)}
                              </button>
                            `,
                          )
                          .join("")}
                      </div>
                    </div>
                  `,
                )
                .join("")
            : `<div class="git-empty">${escapeHtml(t("gitNoDiff"))}</div>`
        }
      </div>
    </section>
    <section class="git-split">
      <article class="git-panel">
        <div class="git-panel-head">${escapeHtml(t("gitDiffWorking"))}</div>
        <div class="git-diff-block markdown-body">${
          !diffLoaded
            ? `<div class="git-empty">${escapeHtml(currentLanguage === "zh" ? "正在加载 diff..." : "Loading diff...")}</div>`
            : workingDiff
              ? renderHighlightedCodeBlock(workingDiff, "diff")
              : `<div class="git-empty">${escapeHtml(t("gitNoDiff"))}</div>`
        }</div>
      </article>
      <article class="git-panel">
        <div class="git-panel-head">${escapeHtml(t("gitDiffStaged"))}</div>
        <div class="git-diff-block markdown-body">${
          !diffLoaded
            ? `<div class="git-empty">${escapeHtml(currentLanguage === "zh" ? "正在加载 diff..." : "Loading diff...")}</div>`
            : stagedDiff
              ? renderHighlightedCodeBlock(stagedDiff, "diff")
              : `<div class="git-empty">${escapeHtml(t("gitNoDiff"))}</div>`
        }</div>
      </article>
    </section>
  `;
}

function renderGitHistory(git) {
  if (!gitHistoryView) return;
  const commits = Array.isArray(git?.commits) ? git.commits : [];
  gitHistoryView.innerHTML = commits.length
    ? `
      <section class="git-panel">
        <div class="git-commit-list git-commit-list-full">
          ${commits
            .map(
              (commit) => `
                <div class="git-commit-item git-commit-item-full">
                  <div class="git-commit-main">
                    <div class="git-commit-subject">${escapeHtml(commit.message || "")}</div>
                    <div class="git-commit-meta">${escapeHtml(commit.hash || "")} / ${escapeHtml(commit.date || "")}</div>
                  </div>
                  <div class="git-commit-author">${escapeHtml(commit.author || "")}</div>
                </div>
              `,
            )
            .join("")}
        </div>
      </section>
    `
    : `<div class="git-empty">${escapeHtml(t("gitHistoryEmpty"))}</div>`;
}

function renderGitBranches(git) {
  if (!gitBranchesView) return;
  const branches = Array.isArray(git?.branches) ? git.branches : [];
  gitBranchesView.innerHTML = branches.length
    ? `
      <section class="git-panel">
        <div class="git-panel-toolbar">
          <button class="git-inline-action" type="button" data-git-create-branch="true">${escapeHtml(t("gitCreateBranch"))}</button>
        </div>
        <div class="git-branch-list">
          ${branches
            .map(
              (branch) => `
                <div class="git-branch-item">
                  <div class="git-branch-main">
                    <div class="git-branch-name">${escapeHtml(branch.name || "")}</div>
                    <div class="git-branch-meta">${escapeHtml(branch.upstream || "")}${branch.last_updated ? ` / ${escapeHtml(branch.last_updated)}` : ""}</div>
                  </div>
                  <div class="git-branch-actions">
                    ${branch.is_current ? `<span class="git-branch-current">${escapeHtml(t("gitBranchCurrent"))}</span>` : ""}
                    ${!branch.is_remote && !branch.is_current ? `<button class="git-branch-action" type="button" data-git-checkout="${escapeHtml(branch.name || "")}">${escapeHtml(t("gitCheckout"))}</button>` : ""}
                    ${!branch.is_remote && !branch.is_current ? `<button class="git-branch-action is-danger" type="button" data-git-delete-branch="${escapeHtml(branch.name || "")}">${escapeHtml(t("gitDeleteBranch"))}</button>` : ""}
                  </div>
                </div>
              `,
            )
            .join("")}
        </div>
      </section>
    `
    : `<div class="git-empty">${escapeHtml(t("gitBranchesEmpty"))}</div>`;
}

const GIT_GRAPH_PALETTE = [
  "#f59e63",
  "#6fb8ff",
  "#8bd39a",
  "#e6b85c",
  "#cb9bff",
  "#ff8f8f",
  "#7ed6c8",
  "#d7c48c",
];

function graphRefNames(row) {
  return Array.isArray(row?.refs) ? row.refs : [];
}

function graphPrimaryRef(row) {
  const refs = graphRefNames(row);
  return refs.find((ref) => /HEAD|->|origin\//.test(ref)) || refs[0] || row?.hash || "";
}

function graphLaneKey(row, fallback) {
  const primary = graphPrimaryRef(row);
  if (primary) {
    return primary
      .replace(/^HEAD ->\s*/i, "")
      .replace(/^tag:\s*/i, "")
      .trim();
  }
  return row?.parents?.[0] || row?.full_hash || fallback;
}

function buildGitGraphRows(rows) {
  const fullToShort = new Map();
  const activeLanes = [];
  const laneColors = [];
  const rendered = [];

  rows.forEach((row) => {
    if (row?.full_hash) {
      fullToShort.set(row.full_hash, row.hash);
    }
  });

  function ensureLaneColor(lane) {
    if (!laneColors[lane]) {
      laneColors[lane] = GIT_GRAPH_PALETTE[lane % GIT_GRAPH_PALETTE.length];
    }
    return laneColors[lane];
  }

  function findFreeLane() {
    let lane = 0;
    while (activeLanes[lane]) lane += 1;
    return lane;
  }

  rows.forEach((row, index) => {
    const commitKey = row?.full_hash || row?.hash || `row-${index}`;
    const refs = graphRefNames(row);
    const branchKey = graphLaneKey(row, commitKey);
    let lane = activeLanes.findIndex((value) => value === commitKey);

    if (lane < 0) {
      lane = activeLanes.findIndex((value) => value === branchKey);
    }
    if (lane < 0) {
      lane = findFreeLane();
    }

    activeLanes[lane] = commitKey;
    const color = ensureLaneColor(lane);
    const parents = Array.isArray(row?.parents) ? row.parents.filter(Boolean) : [];
    const parentLanes = [];

    if (parents.length) {
      const primaryParentHash = parents[0];
      const primaryParentKey = fullToShort.get(primaryParentHash) || primaryParentHash;
      activeLanes[lane] = primaryParentKey;
      parentLanes.push({
        hash: primaryParentHash,
        lane,
        color,
        isPrimary: true,
      });

      parents.slice(1).forEach((parentHash) => {
        const parentKey = fullToShort.get(parentHash) || parentHash;
        let parentLane = activeLanes.findIndex((value) => value === parentKey);
        if (parentLane < 0) {
          parentLane = findFreeLane();
        }
        activeLanes[parentLane] = parentKey;
        parentLanes.push({
          hash: parentHash,
          lane: parentLane,
          color: ensureLaneColor(parentLane),
          isPrimary: false,
        });
      });
    } else {
      activeLanes[lane] = null;
    }

    while (activeLanes.length && !activeLanes[activeLanes.length - 1]) {
      activeLanes.pop();
    }

    rendered.push({
      ...row,
      lane,
      color,
      refs,
      parentLanes,
      laneCount: Math.max(activeLanes.length, lane + 1, 1),
    });
  });

  return rendered;
}

function renderGitGraphLanes(row) {
  const columns = Array.from({ length: row.laneCount }, (_, laneIndex) => {
    const isNodeLane = laneIndex === row.lane;
    const primaryParent = row.parentLanes.find((parent) => parent.isPrimary && parent.lane === laneIndex);
    const mergeParents = row.parentLanes.filter((parent) => !parent.isPrimary && parent.lane === laneIndex);
    const classes = ["git-graph-lane"];
    const style = [];

    if (isNodeLane || primaryParent) {
      classes.push("has-vertical");
      style.push(`--lane-color:${isNodeLane ? row.color : primaryParent.color}`);
    }

    const mergeMarkup = mergeParents
      .map((parent) => {
        const direction = parent.lane < row.lane ? "left" : "right";
        return `<span class="git-graph-merge git-graph-merge-${direction}" style="--merge-color:${parent.color}"></span>`;
      })
      .join("");

    const nodeMarkup = isNodeLane
      ? `<span class="git-graph-node" style="--node-color:${row.color}"></span>`
      : "";

    return `<span class="${classes.join(" ")}" style="${style.join(";")}">${mergeMarkup}${nodeMarkup}</span>`;
  }).join("");

  return `<div class="git-graph-lanes" style="--lane-count:${row.laneCount}">${columns}</div>`;
}

function renderGitGraphRefs(row) {
  if (!Array.isArray(row?.refs) || !row.refs.length) return "";
  return row.refs
    .map((ref, index) => {
      const color = GIT_GRAPH_PALETTE[index % GIT_GRAPH_PALETTE.length];
      return `<span class="git-graph-ref" style="--git-ref-color:${color}">${escapeHtml(ref)}</span>`;
    })
    .join("");
}

function renderGitGraph(git) {
  if (!gitGraphView) return;
  const rows = Array.isArray(git?.graph) ? git.graph : [];
  if (!gitDataLoadState.graph) {
    gitGraphView.innerHTML = `<div class="git-empty">${escapeHtml(currentLanguage === "zh" ? "正在加载图谱..." : "Loading graph...")}</div>`;
    return;
  }
    gitGraphView.innerHTML = `<div class="git-empty">${escapeHtml(currentLanguage === "zh" ? "正在加载图谱..." : "Loading graph...")}</div>`;
  gitGraphView.innerHTML = rows.length
    ? `
      <section class="git-panel git-graph-panel">
        <div class="git-graph-list">
          ${graphRows
            .map(
              (row) => `
                <div class="git-graph-row">
                  ${renderGitGraphLanes(row)}
                  <div class="git-graph-hash">${escapeHtml(row.hash || "")}</div>
                  <div class="git-graph-main">
                    <div class="git-graph-subject">${escapeHtml(row.subject || "")}</div>
                    <div class="git-graph-meta">${renderGitGraphRefs(row)}${row.refs.length ? `<span class="git-graph-meta-sep">/</span>` : ""}${escapeHtml(row.relative_time || "")} / ${escapeHtml(row.author || "")}</div>
                  </div>
                </div>
              `,
            )
            .join("")}
        </div>
      </section>
    `
    : `<div class="git-empty">${escapeHtml(t("gitGraphEmpty"))}</div>`;
}

function renderGitWorkspace(git) {
  if (!gitWorkspace || !gitStatusBanner) return;
  if (!git || git.available === false) {
    gitStatusBanner.innerHTML = `<div class="git-status-error">${escapeHtml(git?.error || t("gitUnavailable"))}</div>`;
    if (gitOverviewView) gitOverviewView.innerHTML = "";
    if (gitChangesView) gitChangesView.innerHTML = "";
    if (gitHistoryView) gitHistoryView.innerHTML = "";
    if (gitBranchesView) gitBranchesView.innerHTML = "";
    if (gitGraphView) gitGraphView.innerHTML = "";
    return;
  }

  gitStatusBanner.innerHTML = `<div class="git-status-copy">${escapeHtml(formatGitStatusSummary(git.status))}</div>`;
  renderGitOverview(git);
  renderGitChanges(git);
  renderGitHistory(git);
  renderGitBranches(git);
  renderGitGraph(git);

  document.querySelectorAll("[data-git-file-action]").forEach((button) => {
    button.addEventListener("click", async () => {
      const action = button.getAttribute("data-git-file-action") || "";
      const path = button.getAttribute("data-git-path") || "";
      if (!action || !path) return;
      try {
        await runGitAction(action, { pathspecs: [path] });
      } catch (error) {
        console.error(error);
        showToast(error?.message || t("gitActionFailed"));
      }
    });
  });

  gitBranchesView.querySelectorAll("[data-git-checkout]").forEach((button) => {
    button.addEventListener("click", async () => {
      const branch = button.getAttribute("data-git-checkout") || "";
      if (!branch) return;
      try {
        await runGitAction("checkout", { branch });
      } catch (error) {
        console.error(error);
        showToast(error?.message || t("gitActionFailed"));
      }
    });
  });

  gitBranchesView.querySelectorAll("[data-git-delete-branch]").forEach((button) => {
    button.addEventListener("click", async () => {
      const branch = button.getAttribute("data-git-delete-branch") || "";
      if (!branch) return;
      if (!window.confirm(template("gitDeleteBranchConfirm", { branch }))) return;
      try {
        await runGitAction("delete_branch", { branch });
      } catch (error) {
        console.error(error);
        showToast(error?.message || t("gitActionFailed"));
      }
    });
  });

  gitBranchesView.querySelectorAll("[data-git-create-branch]").forEach((button) => {
    button.addEventListener("click", async () => {
      const branch = window.prompt(t("gitBranchPrompt"), "");
      if (branch == null || !branch.trim()) return;
      try {
        await runGitAction("create_branch", { branch: branch.trim() });
      } catch (error) {
        console.error(error);
        showToast(error?.message || t("gitActionFailed"));
      }
    });
  });
}

function setMainView(nextView) {
  captureMessageScrollPosition();
  currentMainView = nextView === "git" ? "git" : nextView === "browser" ? "browser" : "chat";
  const workspaceBody = document.getElementById("workspace-body");
  const workspaceChat = document.querySelector(".workspace-chat");
  const conversationStage = document.querySelector(".conversation-stage");
  const composer = document.querySelector(".composer-shell");

  if (workspaceChat) workspaceChat.hidden = currentMainView === "git";
  if (gitWorkspace) gitWorkspace.hidden = currentMainView !== "git";
  if (browserWorkspace) browserWorkspace.hidden = currentMainView !== "browser";
  if (workspaceBody) {
    workspaceBody.classList.toggle("is-browser-split", currentMainView === "browser");
  }
  if (conversationStage) conversationStage.hidden = false;
  if (composer) composer.hidden = currentMainView === "git";
  if (gitNav) {
    gitNav.classList.toggle("is-active", currentMainView === "git");
  }
  applyDockLayout();
  syncLayoutCornerControls();
  requestAnimationFrame(() => restoreMessageScrollPosition());
}

function setGitView(nextView) {
  currentGitView = nextView || "overview";
  if (!gitNav) return;
  gitNav.querySelectorAll("[data-git-view]").forEach((button) => {
    button.classList.toggle("is-active", button.getAttribute("data-git-view") === currentGitView);
  });
  const viewMap = {
    overview: gitOverviewView,
    changes: gitChangesView,
    history: gitHistoryView,
    branches: gitBranchesView,
    graph: gitGraphView,
  };
  Object.entries(viewMap).forEach(([key, element]) => {
    if (!element) return;
    element.hidden = key !== currentGitView;
  });
  const needed = currentGitFetchOptions(currentGitView);
  if ((needed.diff && !gitDataLoadState.diff) || (needed.graph && !gitDataLoadState.graph)) {
    loadGitState(needed).catch((error) => {
      console.error(error);
      showToast(error?.message || t("gitActionFailed"));
    });
  }
}

function renderFromState() {
  if (!bootstrapData) return;
  syncAcceptedDiffStatusesFromMessages(bootstrapData.messages || []);
  syncActiveSessionsFromBootstrap(bootstrapData, {
    preserveRunningSessionId: localStreamingSessionId(bootstrapData?.current_session_id || ""),
  });
  setActivityPanel(activeActivityPanel, { preserveMainView: true });
  setMainView(currentMainView);
  setGitView(currentGitView);
  syncWorkspaceHeader(bootstrapData.workspace_root || bootstrapData.config?.workspace_root || "");
  syncRiskPill(bootstrapData.config);
  syncSettingsFromConfig(bootstrapData.config);
  renderCurrentSession(bootstrapData.sessions || [], bootstrapData.current_session_id);
  renderSessionList(bootstrapData.sessions || [], bootstrapData.current_session_id);
  renderBranches(bootstrapData.branches || []);
  renderProviderList(bootstrapData.config?.providers || [], bootstrapData.config?.model || "");
  renderReview(bootstrapData.review || buildReviewFromMessages(bootstrapData.messages || []));
  syncAgentPreludeBackground(bootstrapData.messages || []);
  if (shouldPreserveStreamingConversationDom()) {
    hydrateVisibleRuntimeSnapshot();
    schedulePendingAssistantTextSync();
    schedulePendingAssistantStatusSync();
  } else {
    renderMessages(bootstrapData.messages || []);
    hydrateVisibleRuntimeSnapshot();
  }
  renderResearch(bootstrapData.research || null);
  renderGitWorkspace(bootstrapData.git || null);
  renderWorkspaceTree(bootstrapData.workspace_browser || null);
  renderExtensionList(extensionSearchInput?.value || "");
  renderSearchPanel();
  renderTerminalDrawer();
}

function applyBootstrap(data) {
  const previousWorkspaceRoot = currentWorkspaceRoot;
  const previousSessionId = String(bootstrapData?.current_session_id || "").trim();
  const previousMessages = visibleConversationMessages(bootstrapData?.messages || []);
  const nextSessionId = String(data?.current_session_id || "").trim();
  const showSandboxNotice = shouldShowSandboxNotice(data?.sandbox);
  const incomingVisibleMessages = visibleConversationMessages(data?.messages || []);
  const incomingSignature = visibleMessagesSignature(incomingVisibleMessages);
  const preserveCompletedVisibleMessages =
    Boolean(
      lastVisibleCompletionSignature &&
      previousSessionId &&
      previousSessionId === nextSessionId &&
      !isVisibleSessionRunning() &&
      incomingSignature &&
      incomingSignature !== lastVisibleCompletionSignature,
    );
  bootstrapData = preserveCompletedVisibleMessages
    ? {
        ...data,
        messages: previousMessages,
      }
    : data;
  if (lastVisibleCompletionSignature && incomingSignature === lastVisibleCompletionSignature) {
    lastVisibleCompletionSignature = "";
  }
  if (previousSessionId !== nextSessionId || !data?.research?.paper_workflow) {
    resetPaperWorkspaceState();
  }
  syncReviewerFeedbackDraftFromResearch(data?.research || null);
  syncAcceptedDiffStatusesFromMessages(bootstrapData?.messages || []);
  setGitLoadState();
  applyHostMeta(data?.host || null);
  currentWorkspaceRoot = String(data?.workspace_root || data?.config?.workspace_root || "");
  if (previousWorkspaceRoot && currentWorkspaceRoot && previousWorkspaceRoot !== currentWorkspaceRoot) {
    disposeWorkspaceMonaco();
    workspaceMonacoModelCache.forEach((model) => {
      try {
        model?.dispose?.();
      } catch (_error) {
        // Ignore disposal failures.
      }
    });
    workspaceMonacoModelCache.clear();
    workspaceMonacoViewStateCache.clear();
    workspaceFileTextCache.clear();
    workspaceSymbolIndexCache.clear();
    workspaceOpenTabs = [];
    workspaceDraftCache.clear();
    workspaceRenderModeByPath.clear();
  }
  const review = data?.review || buildReviewFromMessages(bootstrapData?.messages || []);
  if (!review?.files?.some((file) => file.path === activeReviewFilePath)) {
    activeReviewFilePath = null;
  }

  const bootstrapFiles = workspaceFlattenFiles(data?.workspace_browser?.entries || [], []);
  if (activeWorkspaceFilePath && !bootstrapFiles.some((entry) => entry.path === activeWorkspaceFilePath)) {
    activeWorkspaceFilePath = null;
    isWorkspaceCodeOpen = false;
  }

  renderFromState();
  refreshLatexRendering().catch(() => {});
  if (isWorkspaceCodeOpen && activeWorkspaceFilePath) {
    loadWorkspaceFile(activeWorkspaceFilePath, { preservePanelVisibility: true }).catch((error) => {
      console.error(error);
      renderWorkspaceFile(null);
    });
  } else {
    renderWorkspaceFile(null);
  }
  if (workspacePickerToggle) {
    workspacePickerToggle.disabled = !currentHostMeta.supportsFileDialog;
    workspacePickerToggle.hidden = !currentHostMeta.supportsFileDialog;
  }
  if (terminalRailButton) {
    terminalRailButton.disabled = !currentHostMeta.supportsTerminal;
    terminalRailButton.hidden = !currentHostMeta.supportsTerminal;
  }
  if (showSandboxNotice) {
    markSandboxNoticeShown(data?.sandbox);
    window.setTimeout(() => {
      showToast(t("toastSandboxInitialized"));
    }, 260);
  }
}

function applyTranslations() {
  document.querySelectorAll("[data-i18n]").forEach((node) => {
    const key = node.dataset.i18n;
    if (key) {
      node.textContent = t(key);
    }
  });
  document.querySelectorAll("[data-placeholder-key]").forEach((node) => {
    const key = node.dataset.placeholderKey;
    if (key) {
      node.setAttribute("placeholder", t(key));
    }
  });
  if (langToggle) {
    langToggle.textContent = currentLanguage === "zh" ? "English" : "中文";
  }
  if (sessionMenuRename) sessionMenuRename.textContent = t("renameAction");
  updateEffortUI();
  syncWorkspaceCodeRenderToggle();
  applyWorkspaceMode(currentWorkspaceMode);
  renderExtensionList(extensionSearchInput?.value || "");
  renderSearchPanel();
  setActivityPanel(activeActivityPanel, { preserveMainView: true });
  setMainView(currentMainView);
  setGitView(currentGitView);
  if (bootstrapData) {
    renderFromState();
  }
}

function setLanguage(nextLanguage) {
  currentLanguage = nextLanguage === "en" ? "en" : "zh";
  try {
    localStorage.setItem("tokitai-language", currentLanguage);
  } catch (_error) {
    // ignore
  }
  applyTranslations();
}

async function saveSettings() {
  const reopenSettings = activeSettingsPanel === "settings-panel";
  const reopenTab = activeSettingsTab;
  const response = await hostClient.settings.update(collectSettingsPayload());
  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(errorText || `settings failed: ${response.status}`);
  }
  await loadBootstrap();
  if (reopenSettings) {
    openSettingsPanel("settings-panel");
    setSettingsTab(reopenTab);
  }
  showToast(t("toastSettingsSaved"));
}

async function persistSettingsSilently() {
  const response = await hostClient.settings.update(collectSettingsPayload());
  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(errorText || `settings failed: ${response.status}`);
  }
}

function scheduleEffortPersistence() {
  window.clearTimeout(effortPersistTimer);
  const requestId = ++effortPersistRequestId;
  effortPersistTimer = window.setTimeout(async () => {
    try {
      await persistSettingsSilently();
      if (bootstrapData?.config) {
        bootstrapData.config.reasoning_effort = currentEffort;
      }
    } catch (error) {
      if (requestId !== effortPersistRequestId) return;
      console.error(error);
      showToast(t("toastSendFailed"));
    }
  }, 120);
}

async function pickWorkspace() {
  const response = await hostClient.workspace.pick();
  if (!response.ok) {
    const errorText = await response.text();
    if ((errorText || "").includes("workspace selection cancelled")) {
      showToast(t("toastWorkspaceCanceled"));
      return;
    }
    throw new Error(errorText || `workspace picker failed: ${response.status}`);
  }

  const payload = await response.json();
  const nextRoot = payload?.data?.workspace_root || "";
  if (runtimeWorkspaceRoot && nextRoot) {
    runtimeWorkspaceRoot.value = nextRoot;
  }
  await loadBootstrap();
  const gitOptions = currentGitFetchOptions(currentGitView);
  await Promise.allSettled([
    loadGitState(gitOptions),
    loadRunDebugState(),
    loadTerminalState(),
  ]);
  showToast(t("toastWorkspaceSwitched"));
}

async function loadBootstrap() {
  const response = await hostClient.bootstrap();
  if (!response.ok) {
    throw new Error(`bootstrap failed: ${response.status}`);
  }
  const payload = await response.json();
  setGitLoadState();
  applyBootstrap(payload.data);
  syncActiveSessionsFromBootstrap(payload.data, {
    preserveRunningSessionId: localStreamingSessionId(payload?.data?.current_session_id || ""),
  });
  if (payload?.data?.git) {
    currentMainView = ["git", "browser"].includes(currentMainView) ? currentMainView : currentMainView;
    renderGitWorkspace(payload.data.git);
  }
}

async function refreshBackgroundSessionState() {
  if (pendingBootstrapRefreshPromise) return pendingBootstrapRefreshPromise;
  pendingBootstrapRefreshPromise = (async () => {
    const response = await hostClient.bootstrap();
    if (!response.ok) {
      throw new Error(`bootstrap failed: ${response.status}`);
    }
    const payload = await response.json();
    const nextData = payload?.data || null;
    if (!nextData) return;

    const currentVisibleSessionId = String(bootstrapData?.current_session_id || "").trim();
    const nextVisibleSessionId = String(nextData.current_session_id || "").trim();
    const preservedRunningSessionId = localStreamingSessionId(currentVisibleSessionId || nextVisibleSessionId);
    if (reconcileVisibleSessionCompletion(nextData, {
      sessionId: currentVisibleSessionId || nextVisibleSessionId,
      preserveScroll: true,
    })) {
      return;
    }
    const nextVisibleSessionStillLive = bootstrapHasLiveSession(nextData, currentVisibleSessionId || nextVisibleSessionId);
    const shouldPreserveVisible =
      currentVisibleSessionId &&
      currentVisibleSessionId === nextVisibleSessionId &&
      isVisibleSessionRunning() &&
      nextVisibleSessionStillLive;

    if (shouldPreserveVisible && bootstrapData) {
      const incomingMessages = visibleConversationMessages(nextData.messages || []);
      const incomingSignature = visibleMessagesSignature(incomingMessages);
      const preservedMessages = (
        lastVisibleCompletionSignature &&
        incomingSignature &&
        incomingSignature !== lastVisibleCompletionSignature
      )
        ? incomingMessages
        : (bootstrapData.messages || []);
      const preservedResearch = bootstrapData.research || nextData.research || null;
      bootstrapData = {
        ...nextData,
        messages: preservedMessages,
        research: preservedResearch,
        git: nextData.git || bootstrapData.git || null,
        workspace_browser: nextData.workspace_browser || bootstrapData.workspace_browser || null,
        current_session_id: nextData.current_session_id || bootstrapData.current_session_id || null,
      };
      syncActiveSessionsFromBootstrap(nextData, {
        preserveRunningSessionId: preservedRunningSessionId,
      });
      renderCurrentSession(bootstrapData.sessions || [], bootstrapData.current_session_id || null);
      renderSessionList(bootstrapData.sessions || [], bootstrapData.current_session_id || null);
      renderBranches(bootstrapData.branches || []);
      renderProviderList(bootstrapData.config?.providers || [], bootstrapData.config?.model || "");
      syncWorkspaceHeader(bootstrapData.workspace_root || bootstrapData.config?.workspace_root || "");
      syncRiskPill(bootstrapData.config);
      syncSettingsFromConfig(bootstrapData.config);
      renderResearch(bootstrapData.research || null);
      renderGitWorkspace(bootstrapData.git || null);
      renderWorkspaceTree(bootstrapData.workspace_browser || null);
      renderExtensionList(extensionSearchInput?.value || "");
      renderTerminalDrawer();
      if (isVisibleSessionRunning()) {
        hydrateVisibleRuntimeSnapshot();
      }
      return;
    }

    setGitLoadState();
    applyBootstrap(nextData);
    syncActiveSessionsFromBootstrap(nextData, {
      preserveRunningSessionId: preservedRunningSessionId,
    });
    if (nextData?.git) {
      currentMainView = ["git", "browser"].includes(currentMainView) ? currentMainView : currentMainView;
      renderGitWorkspace(nextData.git);
    }
  })();

  try {
    await pendingBootstrapRefreshPromise;
  } finally {
    pendingBootstrapRefreshPromise = null;
  }
}

async function loadGitState(options = {}) {
  const requestOptions = {
    diff: options.diff === true,
    graph: options.graph === true,
  };
  if (
    bootstrapData?.git &&
    (!requestOptions.diff || gitDataLoadState.diff) &&
    (!requestOptions.graph || gitDataLoadState.graph) &&
    !options.force
  ) {
    renderGitWorkspace(bootstrapData?.git || null);
    return;
  }
  if (gitLoadPromise) {
    return gitLoadPromise;
  }
  gitLoadPromise = (async () => {
    const response = await hostClient.git.state(requestOptions);
    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(errorText || `git state failed: ${response.status}`);
    }
    const payload = await response.json();
    const nextGit = payload?.data?.git || null;
    const previousGit = bootstrapData?.git || null;
    const mergedGit = nextGit
      ? {
          ...(previousGit || {}),
          ...nextGit,
          staged_diff:
            requestOptions.diff || nextGit?.staged_diff != null
              ? nextGit?.staged_diff ?? null
              : previousGit?.staged_diff ?? null,
          working_diff:
            requestOptions.diff || nextGit?.working_diff != null
              ? nextGit?.working_diff ?? null
              : previousGit?.working_diff ?? null,
          graph:
            requestOptions.graph || Array.isArray(nextGit?.graph)
              ? nextGit?.graph || []
              : previousGit?.graph || [],
        }
      : null;
    bootstrapData = {
      ...(bootstrapData || {}),
      git: mergedGit,
    };
    gitDataLoadState = {
      diff: gitDataLoadState.diff || requestOptions.diff,
      graph: gitDataLoadState.graph || requestOptions.graph,
    };
    renderGitWorkspace(bootstrapData?.git || null);
  })();
  try {
    await gitLoadPromise;
  } finally {
    gitLoadPromise = null;
  }
}

async function loadExtensions() {
  const response = await hostClient.extensions.list();
  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(errorText || `extensions failed: ${response.status}`);
  }
  const payload = await response.json();
  extensionCatalog = Array.isArray(payload?.data?.extensions?.items) ? payload.data.extensions.items : [];
  renderExtensionList(extensionSearchInput?.value || "");
}

async function loadRunDebugState() {
  const response = await hostClient.runDebug.state();
  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(errorText || `run/debug failed: ${response.status}`);
  }
  const payload = await response.json();
  runDebugState = payload?.data?.run_debug || null;
  renderRunDebug(runDebugState);
}

async function loadTerminalState() {
  const response = await hostClient.terminals.state();
  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(errorText || `terminal failed: ${response.status}`);
  }
  const payload = await response.json();
  terminalState = payload?.data?.terminals || { sessions: [], active_id: null };
  renderTerminalDrawer();
}

async function createTerminal() {
  const response = await hostClient.terminals.create();
  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(errorText || `terminal create failed: ${response.status}`);
  }
  const payload = await response.json();
  terminalState = payload?.data?.terminals || { sessions: [], active_id: null };
  terminalDrawerDismissed = false;
  renderTerminalDrawer();
}

async function sendTerminalInput(input) {
  const active = getActiveTerminal();
  if (!active) return;
  const response = await hostClient.terminals.input(active.id, input);
  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(errorText || `terminal input failed: ${response.status}`);
  }
  const payload = await response.json();
  terminalState = payload?.data?.terminals || { sessions: [], active_id: null };
  renderTerminalDrawer();
}

async function closeTerminal(terminalId) {
  const response = await hostClient.terminals.close(terminalId);
  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(errorText || `terminal close failed: ${response.status}`);
  }
  const payload = await response.json();
  terminalState = payload?.data?.terminals || { sessions: [], active_id: null };
  if (!Array.isArray(terminalState.sessions) || !terminalState.sessions.length) {
    terminalDrawerDismissed = false;
  }
  renderTerminalDrawer();
}

async function runRunDebugAction(action, configId = null) {
  const response = await hostClient.runDebug.action(action, configId);
  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(errorText || `run/debug action failed: ${response.status}`);
  }
  const payload = await response.json();
  runDebugState = payload?.data?.run_debug || null;
  renderRunDebug(runDebugState);
}

async function runGitAction(action, extra = {}) {
  const requested = currentGitFetchOptions(currentGitView);
  const response = await hostClient.git.action(action, {
    ...extra,
    diff: requested.diff,
    graph: requested.graph,
  });
  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(errorText || `git action failed: ${response.status}`);
  }
  const payload = await response.json();
  gitDataLoadState = {
    diff: requested.diff,
    graph: requested.graph,
  };
  bootstrapData = {
    ...(bootstrapData || {}),
    git: payload?.data?.git || null,
  };
  renderGitWorkspace(bootstrapData?.git || null);
}

async function createSession() {
  const response = await hostClient.sessions.create();
  if (!response.ok) {
    throw new Error(`create session failed: ${response.status}`);
  }
  resetConversationRuntimeState({ preserveInputFocus: true });
  await loadBootstrap();
  messageInput?.focus();
  showToast(t("toastSessionCreated"));
}

async function switchSession(sessionId) {
  const response = await hostClient.sessions.select(sessionId);
  if (!response.ok) {
    throw new Error(`switch session failed: ${response.status}`);
  }
  resetConversationRuntimeState({ preserveInputFocus: true });
  await loadBootstrap();
  showToast(t("toastSessionSwitched"));
}

async function renameSession(sessionId, title) {
  const response = await hostClient.sessions.rename(sessionId, title);
  if (!response.ok) {
    throw new Error(`rename session failed: ${response.status}`);
  }
  await loadBootstrap();
}

async function deleteSession(sessionId) {
  const response = await hostClient.sessions.delete(sessionId);
  if (!response.ok) {
    throw new Error(`delete session failed: ${response.status}`);
  }
  unmarkResearchStartedForSession(sessionId);
  paperWorkflowPendingSessions.delete(String(sessionId || "").trim());
  paperWorkflowAutoTriggeredSessions.delete(String(sessionId || "").trim());
  paperWorkflowPromptDismissedSessions.delete(String(sessionId || "").trim());
  await loadBootstrap();
}

async function consumeStream(response, sessionId) {
  const runState = getSessionRunState(sessionId);
  const streamGeneration = runState?.generation ?? activeStreamGeneration;
  if (currentHostMeta.mode === "desktop" && currentHostMeta.transport === "bridge") {
    const stream = response;
    if (!stream || typeof stream[Symbol.asyncIterator] !== "function") {
      throw new Error("desktop stream transport is unavailable");
    }

    for await (const rawChunk of stream) {
      const currentState = getSessionRunState(sessionId);
      if (!currentState || streamGeneration !== currentState.generation) {
        if (typeof stream.return === "function") {
          try {
            await stream.return();
          } catch (_error) {
            // noop
          }
        }
        return;
      }
      if (!rawChunk) continue;
      const event = typeof rawChunk === "string" ? JSON.parse(rawChunk) : rawChunk;
      handleStreamEvent(event);
    }
    const finalState = getSessionRunState(sessionId);
    if (finalState?.running) {
      await refreshBackgroundSessionState().catch(() => {});
      if (!getSessionRunState(sessionId)?.running) {
        materializePendingConversationMessages({ sessionId });
        endSessionRun(sessionId);
      }
    }
    return;
  }

  const reader = response.body?.getReader();
  if (!reader) {
    throw new Error("Streaming is unavailable in this environment.");
  }

  const decoder = new TextDecoder();
  let buffer = "";

  while (true) {
    const { value, done } = await reader.read();
    const currentState = getSessionRunState(sessionId);
    if (!currentState || streamGeneration !== currentState.generation) {
      try {
        await reader.cancel();
      } catch (_error) {
        // noop
      }
      return;
    }

    if (value) {
      buffer += decoder.decode(value, { stream: !done });
      let newlineIndex = buffer.indexOf("\n");
      while (newlineIndex !== -1) {
        const line = buffer.slice(0, newlineIndex).trim();
        buffer = buffer.slice(newlineIndex + 1);

        if (line) {
          const event = JSON.parse(line);
          handleStreamEvent(event, sessionId);
        }

        newlineIndex = buffer.indexOf("\n");
      }
    }

    if (done) break;
  }

  const finalState = getSessionRunState(sessionId);
  if (finalState?.running) {
    await refreshBackgroundSessionState().catch(() => {});
    if (!getSessionRunState(sessionId)?.running) {
      materializePendingConversationMessages({ sessionId });
      endSessionRun(sessionId);
    }
  }
}

function handleStreamEvent(event, expectedSessionId = null) {
  if (!event) return;
  const sessionId = String(event.session_id || expectedSessionId || "").trim();
  const isVisibleSession = sessionId && sessionId === String(bootstrapData?.current_session_id || "").trim();

  if (sessionId) {
    touchSessionRun(sessionId, { running: true });
  }

  if (event.session_id && isVisibleSession) {
    currentStreamingSessionId = event.session_id;
    if (pendingResearchStart && currentWorkspaceMode === "research") {
      markResearchStartedForSession(event.session_id);
    }
  }

  if (event.research && isVisibleSession) {
    bootstrapData = {
      ...(bootstrapData || {}),
      research: event.research,
    };
    renderResearch(event.research);
  }

  if (event.type === "thinking_delta") {
    if (!isVisibleSession) return;
    if (!activeAssistantTurn) {
      resetActiveAssistantTurn();
    }
    activeAssistantTurn.isThinkingPhase = true;
    completeContextCompactionMoment();
    const delta = event.thinking_delta || "";
    if (!delta.trim()) return;
    appendThinkingContent(delta);
    if (!pendingAssistantBubble) {
      pendingAssistantBubble = appendAssistantBubble("");
    }
    refreshPendingAssistantBubble();
    return;
  }

  if (event.type === "assistant_delta") {
    if (!isVisibleSession) return;
    if (!activeAssistantTurn) {
      resetActiveAssistantTurn();
    }
    activeAssistantTurn.isThinkingPhase = false;
    completeContextCompactionMoment();
    activeAssistantTurn.activity = "";
    updateAssistantBubble(event.delta || "");
    return;
  }

  if (event.type === "assistant_progress") {
    if (!isVisibleSession) return;
    if (!activeAssistantTurn) {
      resetActiveAssistantTurn();
    }
    pushAssistantProgressWorklogText(event.delta || "");
    updateRuntimeNarration(event.delta || "");
    refreshPendingAssistantBubble();
    schedulePendingAssistantStatusSync();
    return;
  }

  if (event.type === "messages" || event.type === "complete") {
    const rawVisibleMessages = visibleConversationMessages(event.messages || []);
    if (event.type === "complete") {
      const finalizedRuntimeTurn = cloneAssistantTurnState(activeAssistantTurn);
      if (activeAssistantTurn) {
        pushAssistantWorklog(describeCompletionWorklog(event));
      }
      suppressVisibleStreamBootstrap = false;
      if (event.research?.runtime) {
        syncActiveTurnRuntime(event.research.runtime);
      }
      const mergedRuntimeTurn = mergeAssistantTurnData(activeAssistantTurn, finalizedRuntimeTurn, {
        preferLiveText: true,
      });
      const hasAssistantMessageInPayload = rawVisibleMessages.some((message) =>
        (message && message.kind === "message" && message.role === "assistant" && cleanDisplayText(String(message.content || "").trim(), ""))
        || messageHasAssistantChoices(message)
      );
      const visibleMessages = hasAssistantMessageInPayload
        ? rawVisibleMessages
        : ensureVisibleAssistantCompletionMessage(rawVisibleMessages, mergedRuntimeTurn);
      if (sessionId) {
        const targetSession = (bootstrapData?.sessions || []).find((session) => session.id === sessionId);
        if (targetSession) {
          targetSession.updated_at = new Date().toISOString();
          targetSession.message_count = visibleMessages.length;
          const nextSummary = latestConversationSummary(visibleMessages, 42);
          targetSession.summary = nextSummary || targetSession.summary || "";
        }
      }
      if (sessionId) {
        endSessionRun(sessionId);
      }
      if (!isVisibleSession) {
        return;
      }
      bootstrapData = {
        ...(bootstrapData || {}),
        messages: visibleMessages,
        current_session_id: event.session_id || bootstrapData?.current_session_id || null,
      };
      syncAcceptedDiffStatuses(mergedRuntimeTurn);
      clearVisibleRuntimeSnapshot(sessionId);
      finalizeActiveAssistantTurn();
      pendingPermissionRequest = null;
      liveToolEvents = [];
      liveEditedFiles = [];
      liveProcessEvents = [];
      renderAgentRuntimeStrip();
      renderAgentProcessStrip();
      renderPermissionStrip();
      renderReview(buildReviewFromMessages(visibleMessages));
      syncAgentPreludeBackground(visibleMessages);
      const finalizedInPlace = finalizeVisibleAssistantBubble(visibleMessages, mergedRuntimeTurn);
      if (!finalizedInPlace) {
        renderMessages(visibleMessages);
      }
      refreshBackgroundSessionState().catch(() => {});
      return;
    }

    const visibleMessages = rawVisibleMessages;
    if (sessionId) {
      const targetSession = (bootstrapData?.sessions || []).find((session) => session.id === sessionId);
      if (targetSession) {
        targetSession.updated_at = new Date().toISOString();
        targetSession.message_count = visibleMessages.length;
        const nextSummary = latestConversationSummary(visibleMessages, 42);
        targetSession.summary = nextSummary || targetSession.summary || "";
      }
    }

    if (!isVisibleSession) {
      return;
    }
    bootstrapData = {
      ...(bootstrapData || {}),
      messages: visibleMessages,
      current_session_id: event.session_id || bootstrapData?.current_session_id || null,
    };
    if (pendingAssistantBubble && shouldPreferPersistedAssistantTurn(visibleMessages)) {
      const finalizedInPlace = finalizeVisibleAssistantBubble(visibleMessages, activeAssistantTurn);
      if (!finalizedInPlace) {
        renderMessages(visibleMessages, { preserveScroll: true });
      }
      finalizeActiveAssistantTurn();
      pendingPermissionRequest = null;
      liveToolEvents = [];
      liveEditedFiles = [];
      liveProcessEvents = [];
      renderAgentRuntimeStrip();
      renderAgentProcessStrip();
      renderPermissionStrip();
    }
    return;
  }

  if (event.type === "activity") {
    if (sessionId) {
      touchSessionRun(sessionId, {
        running: true,
        waitingApproval: event.activity?.label === "permission_required",
      });
    }
    if (!isVisibleSession) return;
    if (!activeAssistantTurn) {
      resetActiveAssistantTurn();
    }
    const label = event.activity?.label || "";
    const detail = event.activity?.detail || "";
    const phase = event.activity?.phase || "";
    const status = event.activity?.status || "";
    const meta = event.activity?.meta || "";
    const agent = event.activity?.agent || "";
    if (label === "context_usage") {
      updateContextUsage(Number(detail || 0), Number(meta || 0));
      return;
    }
    if (label === "context_compaction") {
      pushAssistantStreamMoment({
        kind: "compaction",
        text: detail || (currentLanguage === "zh" ? "??????" : "Auto-compacting context"),
        state: status === "complete" ? "done" : "run",
        dedupeKey: "context-compaction",
        timestamp: Date.now(),
      });
      refreshPendingAssistantBubble();
      return;
    }
    addProcessEvent(
      label || "activity",
      label,
      detail,
      meta,
      { phase, status, agent },
    );
    if (activeAssistantTurn) {
      activeAssistantTurn.processDelegates = Array.isArray(event.activity?.delegates)
        ? event.activity.delegates.map((delegate) => ({ ...delegate }))
        : [];
    }
    if (event.research?.runtime) {
      syncActiveTurnRuntime(event.research.runtime);
      refreshPendingAssistantBubble();
    }
    updateRuntimeNarration(describeActivityNarration({
      label,
      detail,
      meta,
      phase,
      status,
      agent,
    }));
    if (label === "permission_required" && activeAssistantTurn) {
      activeAssistantTurn.activity = currentLanguage === "zh" ? "等待批准" : "Awaiting approval";
    }
    schedulePendingAssistantStatusSync();
    return;
  }

  if (event.type === "subagent") {
    if (!isVisibleSession) return;
    if (!activeAssistantTurn) {
      resetActiveAssistantTurn();
    }
    const items = Array.isArray(event.subagents) ? event.subagents : [];
    items.forEach((subagent) => {
      const id = String(subagent?.id || "").trim() || `${Date.now()}`;
      const existing = activeAssistantTurn.subagents.find((item) => String(item.id || "") === id);
      if (existing) {
        Object.assign(existing, subagent);
      } else {
        activeAssistantTurn.subagents.push({ ...subagent, id });
      }
      pushAssistantWorklog(describeSubagentWorklog({ ...subagent, id }));
      updateRuntimeNarration(subagent?.output || subagent?.purpose || subagent?.name || "");
    });
    refreshPendingAssistantBubble();
    if (event.research?.runtime) {
      syncActiveTurnRuntime(event.research.runtime);
    }
    schedulePendingAssistantStatusSync();
    return;
  }

  if (event.type === "verifier") {
    if (!isVisibleSession) return;
    if (!activeAssistantTurn) {
      resetActiveAssistantTurn();
    }
    activeAssistantTurn.verifierReport = event.verifier ? { ...event.verifier } : null;
    pushAssistantWorklog(describeVerifierWorklog(activeAssistantTurn.verifierReport));
    updateRuntimeNarration(activeAssistantTurn.verifierReport?.summary || "");
    refreshPendingAssistantBubble();
    if (event.research?.runtime) {
      syncActiveTurnRuntime(event.research.runtime);
    }
    schedulePendingAssistantStatusSync();
    return;
  }

  if (event.type === "tool") {
    const tool = event.tool || null;
    if (!tool) return;
    if (!isVisibleSession) return;
    if (!activeAssistantTurn) {
      resetActiveAssistantTurn();
    }
    completeContextCompactionMoment();
    upsertToolEntry(tool);
    pushAssistantWorklog(describeToolWorklog(tool));
    pushAssistantStreamMoment(normalizedOperationMoment(describeToolMoment(tool), tool));
    updateRuntimeNarration(summarizeRuntimeToolNarration(tool));
    liveToolEvents = [...liveToolEvents.filter((item) => item.call_id !== tool.call_id), tool].slice(-6);
    refreshPendingAssistantBubble();
    renderAgentRuntimeStrip();
    if (String(tool.name || "").startsWith("terminal_")) {
      terminalDrawerDismissed = false;
      loadTerminalState()
        .then(() => scheduleTerminalPoll(1200))
        .catch(() => {});
    }
    schedulePendingAssistantStatusSync();
    return;
  }

  if (event.type === "edited_files") {
    const files = Array.isArray(event.edited_files) ? event.edited_files : [];
    if (!files.length) return;
    if (!isVisibleSession) return;
    if (!activeAssistantTurn) {
      resetActiveAssistantTurn();
    }
    files.forEach((file) => upsertDiffEntry(file));
    files.forEach((file) => {
      pushAssistantWorklog(describeEditedFileWorklog(file));
      pushAssistantStreamMoment(normalizedOperationMoment(describeEditedFileMoment(file), file));
      updateRuntimeNarration(summarizeRuntimeDiffNarration(file));
    });
    liveEditedFiles = [...liveEditedFiles, ...files].slice(-6);
    refreshPendingAssistantBubble();
    renderAgentRuntimeStrip();
    schedulePendingAssistantStatusSync();
    return;
  }

  if (event.type === "permission_required") {
    if (sessionId) {
      touchSessionRun(sessionId, { running: true, waitingApproval: true });
    }
    if (!isVisibleSession) return;
    pendingPermissionRequest = event.permission || null;
    if (!activeAssistantTurn) {
      resetActiveAssistantTurn();
    }
    if (activeAssistantTurn) {
      activeAssistantTurn.permission = pendingPermissionRequest;
      activeAssistantTurn.activity = "Awaiting approval";
    }
    const approvalText = cleanDisplayText(
      pendingPermissionRequest?.reason
      || pendingPermissionRequest?.name
      || "",
      "",
    );
    if (approvalText) {
      pushAssistantWorklog({
        kind: "approval",
        text: approvalText,
        dedupeKey: `permission:required:${approvalText}`,
      });
      updateRuntimeNarration(approvalText);
    }
    refreshPendingAssistantBubble();
    renderPermissionStrip();
    schedulePendingAssistantStatusSync();
    return;
  }

  if (event.type === "error") {
    if (Array.isArray(event.messages) && event.messages.length) {
      persistConversationMessages(event.messages, { sessionId });
    }
    if (sessionId) {
      endSessionRun(sessionId);
    }
    throw new Error(event.error || "stream failed");
  }
}

function updateContextUsage(usedTokens, contextWindow) {
  if (!contextUsage || !contextUsageRing || !contextUsageLabel) return;
  const used = Math.max(0, Number(usedTokens || 0));
  const limit = Math.max(1, Number(contextWindow || 128000));
  const percent = Math.min(100, Math.max(0, (used / limit) * 100));
  contextUsage.style.setProperty("--context-usage", `${percent * 3.6}deg`);
  contextUsageLabel.textContent = `${Math.round(percent)}%`;
  contextUsage.title = `${used.toLocaleString()} / ${limit.toLocaleString()} tokens`;
  contextUsage.classList.toggle("is-warning", percent >= 70 && percent < 90);
  contextUsage.classList.toggle("is-critical", percent >= 90);
}

async function sendMessageFallback(content) {
  const response = await hostClient.chat.send(content, currentLanguage);

  if (!response.ok) {
    const errText = await response.text();
    throw new Error(errText || `send failed: ${response.status}`);
  }

  const payload = await response.json();
  persistConversationMessages(payload?.data?.messages || [], {
    sessionId: payload?.data?.session_id || null,
  });
}

async function sendMessage() {
  const content = String(messageInput?.value || "").trim();
  if ((!content && !pendingFiles.length) || isSending) return;
  const parsedInput = parseAgentInputProtocol(content);
  const targetSessionId = await ensureSessionReady();
  if (!targetSessionId) {
    showToast(classifyAppError(new Error("session not ready"), "send").message);
    return;
  }

  isSending = true;
  suppressVisibleStreamBootstrap = true;
  if (messageInput) messageInput.disabled = true;
  setStopButtonVisible(true);
  startActivity(currentLanguage === "zh" ? "思考中" : "Thinking");
  liveEditedFiles = [];
  liveProcessEvents = [];
  pinnedEditedFiles = [];
  pendingPermissionRequest = null;
  renderAgentRuntimeStrip();
  renderAgentProcessStrip();
  renderPermissionStrip();

  try {
    if (!parsedInput.outbound) {
      if (currentWorkspaceMode === "research") {
        showToast(currentLanguage === "zh" ? "请在 /spec 后补充研究主题。" : "Add a research topic after /spec.");
      }
      throw new Error("empty outbound content");
    }
    const userText = sanitizeMessageContent(parsedInput.display);
    const outbound = parsedInput.outbound.trim();
    const attachments = await serializePendingAttachments();
    const mode = parsedInput.mode;
    pendingResearchStart = parsedInput.forceResearch || mode === "research" || currentWorkspaceMode === "research";

    if (pendingUserBubble) pendingUserBubble.remove();
    pendingUserBubble = appendUserBubble(userText);

    if (pendingAssistantBubble) pendingAssistantBubble.remove();
    resetPendingAssistantRenderState();
    clearPendingAssistantFrames();
    resetActiveAssistantTurn();
    preservedThinking = [];
    activeAssistantTurn.startedAt = Date.now();
    activeAssistantTurn.isThinkingPhase = true;
    activeAssistantTurn.activity = mode === "research"
      ? (currentLanguage === "zh" ? "研究中" : "Researching")
      : t("activityReviewing");
    pendingAssistantBubble = appendAssistantBubble("");
    await waitForNextBrowserPaint();
    bootstrapData = {
      ...(bootstrapData || {}),
      messages: [
        ...visibleConversationMessages(bootstrapData?.messages || []),
        { kind: "message", role: "user", content: userText },
      ],
    };
    renderReview(buildReviewFromMessages(bootstrapData.messages || []));
    syncAgentPreludeBackground(bootstrapData.messages || []);

    const streamGeneration = beginSessionRun(targetSessionId);
    activeStreamGeneration = streamGeneration || activeStreamGeneration + 1;
    currentStreamingSessionId = targetSessionId;
    const response = await hostClient.chat.stream({ content: outbound, mode, language: currentLanguage, attachments });

    if (messageInput) {
      messageInput.value = "";
    }
    pendingFiles.forEach((file) => file.previewUrl && URL.revokeObjectURL(file.previewUrl));
    pendingFiles = [];
    if (fileInput) {
      fileInput.value = "";
    }
    renderPendingFiles();

    if (!response.ok) {
      const errText = await response.text();
      throw new Error(errText || `send failed: ${response.status}`);
    }

    try {
      await consumeStream(response, targetSessionId);
    } catch (streamError) {
      console.error(streamError);
      throw streamError;
    }
  } catch (error) {
    console.error(error);
    pendingResearchStart = false;
    suppressVisibleStreamBootstrap = false;
    if (error?.message === "empty outbound content") {
      // Input protocol validation already showed a toast.
    } else {
      const classified = classifyAppError(error, "send");
      commitStreamFailure(error);
      showToast(classified.message);
    }
  } finally {
    pendingResearchStart = false;
    suppressVisibleStreamBootstrap = false;
    const stillRunning = Boolean(targetSessionId && getSessionRunState(targetSessionId)?.running);
    if (targetSessionId) {
      const state = getSessionRunState(targetSessionId);
      if (state && !state.running) {
        state.waitingApproval = false;
      }
    }
    renderAgentProcessStrip();
    isSending = false;
    if (stillRunning) {
      currentStreamingSessionId = targetSessionId;
      setStopButtonVisible(true);
      renderActivity();
    } else {
      currentStreamingSessionId = null;
      setStopButtonVisible(false);
      pendingPermissionRequest = null;
      stopActivity();
    }
    renderPermissionStrip();
    if (messageInput) {
      messageInput.disabled = false;
      messageInput.focus();
    }
  }
}

settingsTabs.forEach((tab) => {
  tab.addEventListener("click", () => setSettingsTab(tab.dataset.settingsTab));
});

settingsPanel?.addEventListener("click", (event) => {
  event.stopPropagation();
});

effortButtons.forEach((button) => {
  button.addEventListener("click", (event) => {
    event.stopPropagation();
    const nextEffort = (button.dataset.effort || "medium").toLowerCase();
    if (nextEffort === currentEffort) return;
    currentEffort = nextEffort;
    updateEffortUI();
    if (bootstrapData?.config) {
      bootstrapData.config.reasoning_effort = currentEffort;
    }
    scheduleEffortPersistence();
  });
});

segmentedControls.forEach((group) => {
  const initial = group.querySelector(".segment.is-active")?.dataset.value
    || group.querySelector(".segment")?.dataset.value
    || "";
  setSegmentedValue(group, initial);
  group.querySelectorAll(".segment").forEach((button) => {
    button.addEventListener("click", () => {
      if (group === riskBoundary && autoApproveTools && !autoApproveTools.checked) {
        autoApproveTools.checked = true;
        syncAutoApproveUI();
      }
      setSegmentedValue(group, button.dataset.value || "");
    });
  });
});

autoApproveTools?.addEventListener("change", () => {
  syncAutoApproveUI();
});

autoOpenActivityPanelToggle?.addEventListener("change", () => {
  autoOpenActivityPanel = Boolean(autoOpenActivityPanelToggle.checked);
  try {
    localStorage.setItem("tokitai-auto-open-activity-panel", autoOpenActivityPanel ? "true" : "false");
  } catch (_error) {
    // Ignore storage failures.
  }
  if (pendingAssistantRuntimeNode) {
    syncPendingAssistantRuntimePanel();
  }
  if (!autoOpenActivityPanel) {
    document.querySelectorAll("[data-runtime-panel]").forEach((panel) => {
      panel.closest(".codex-runtime-panel")?.remove();
    });
  } else if (!isVisibleSessionRunning()) {
    renderMessages(visibleConversationMessages(bootstrapData?.messages || []), { preserveScroll: true });
  }
});

researchFloatingHead?.addEventListener("pointerdown", startResearchFloatingDrag);
researchFloatingBoard?.addEventListener("dblclick", (event) => {
  if (event.target instanceof HTMLElement && event.target.closest("button")) return;
  setResearchDetailOpen(true);
});
researchFloatingHide?.addEventListener("pointerdown", (event) => {
  event.stopPropagation();
});
researchFloatingHide?.addEventListener("click", (event) => {
  event.preventDefault();
  event.stopPropagation();
  dismissResearchFloatingBoard();
});
researchFloatingReopen?.addEventListener("click", () => {
  if (researchFloatingReopenSuppressClick) {
    researchFloatingReopenSuppressClick = false;
    return;
  }
  researchFloatingDismissed = false;
  researchFloatingBoardPosition = null;
  syncResearchFloatingBoardPositionFromButton();
  renderResearchFloatingBoard(bootstrapData?.research || null);
});
researchFloatingReopen?.addEventListener("pointerdown", startResearchFloatingReopenDrag);
document.addEventListener("pointermove", moveResearchFloatingDrag);
document.addEventListener("pointerup", endResearchFloatingDrag);
document.addEventListener("pointermove", moveResearchFloatingReopenDrag);
document.addEventListener("pointerup", endResearchFloatingReopenDrag);

document.addEventListener("click", (event) => {
  const paperWorkflowButton = event.target instanceof HTMLElement
    ? event.target.closest("[data-paper-workflow-run]")
    : null;
  if (paperWorkflowButton) {
    event.preventDefault();
    runPaperWorkflow().catch((error) => {
      console.error(error);
      showToast(error?.message || t("toastSendFailed"));
    });
    return;
  }

  const paperWorkflowDismissButton = event.target instanceof HTMLElement
    ? event.target.closest("[data-paper-workflow-dismiss]")
    : null;
  if (paperWorkflowDismissButton) {
    event.preventDefault();
    const sessionId = cleanDisplayText(
      paperWorkflowDismissButton.getAttribute("data-paper-workflow-dismiss") || "",
      String(bootstrapData?.current_session_id || "").trim(),
    );
    if (sessionId) {
      paperWorkflowPromptDismissedSessions.add(sessionId);
      renderMessages(bootstrapData?.messages || [], { preserveScroll: true });
      showToast(t("toastPaperWorkflowDismissed"));
    }
    return;
  }

  const feedbackRefreshButton = event.target instanceof HTMLElement
    ? event.target.closest("[data-reviewer-feedback-refresh]")
    : null;
  if (feedbackRefreshButton) {
    event.preventDefault();
    refreshReviewerFeedback().catch((error) => {
      console.error(error);
      showToast(error?.message || t("toastSendFailed"));
    });
    return;
  }

  const feedbackResolveButton = event.target instanceof HTMLElement
    ? event.target.closest("[data-reviewer-feedback-resolve]")
    : null;
  if (feedbackResolveButton) {
    event.preventDefault();
    const rawIndex = feedbackResolveButton.getAttribute("data-reviewer-feedback-resolve") || "";
    const index = Number(rawIndex);
    if (Number.isFinite(index) && index >= 0) {
      resolveReviewerFeedback(index).catch((error) => {
        console.error(error);
        showToast(error?.message || t("toastSendFailed"));
      });
    }
    return;
  }

  if (
    effortDisclosure &&
    effortDisclosure.hasAttribute("open") &&
    !effortDisclosure.contains(event.target)
  ) {
    closeEffortPanel();
  }

  if (
    activeSettingsPanel &&
    !document.getElementById("settings-panel")?.contains(event.target) &&
    !settingsToggle?.contains(event.target)
  ) {
    closeSettingsPanels();
  }

  if (
    !event.target.closest(".session-actions") &&
    !event.target.closest(".session-floating-menu")
  ) {
    closeSessionMenus();
    closePanelMenu();
  }
});

document.addEventListener("keydown", (event) => {
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
    const hasEditor = Boolean(workspaceMonacoEditor || document.getElementById("workspace-code-editor"));
    if (hasEditor && workspaceEditorDirty) {
      event.preventDefault();
      saveWorkspaceFile().catch((error) => {
        console.error(error);
        showToast(error?.message || t("toastSendFailed"));
      });
      return;
    }
  }
  if (event.key === "Escape") {
    closeEffortPanel();
    closeSettingsPanels();
    closeSessionMenus();
    closePanelMenu();
    stopDockDrag();
    stopResizerDrag();
  }
});

document.addEventListener("input", (event) => {
  const field = event.target instanceof HTMLElement
    ? event.target.closest("[data-reviewer-feedback-field]")
    : null;
  if (!field) return;
  const fieldName = field.getAttribute("data-reviewer-feedback-field") || "";
  if (!fieldName) return;
  const value = "value" in field ? field.value : "";
  updateReviewerFeedbackDraft(fieldName, value);
});

document.addEventListener("submit", (event) => {
  const form = event.target instanceof HTMLElement
    ? event.target.closest("[data-reviewer-feedback-form]")
    : null;
  if (!form) return;
  event.preventDefault();
  submitReviewerFeedback().catch((error) => {
    console.error(error);
    showToast(error?.message || t("toastSendFailed"));
  });
});

window.addEventListener("resize", () => {
  captureMessageScrollPosition();
  stopDockDrag();
  stopResizerDrag();
  syncShellLayoutVars();
  applyDockLayout();
  if (activeSessionMenuAnchor && !sessionMenu?.hidden) {
    positionSessionMenu(activeSessionMenuAnchor);
  }
  requestAnimationFrame(() => restoreMessageScrollPosition());
});

window.addEventListener(
  "scroll",
  () => {
    if (activeSessionMenuAnchor && !sessionMenu?.hidden) {
      positionSessionMenu(activeSessionMenuAnchor);
    }
  },
  true,
);

document.addEventListener("visibilitychange", () => {
  scheduleTerminalPoll(document.hidden ? 3000 : 1500);
});

window.addEventListener("pointermove", onDockPointerMove);
window.addEventListener("pointerup", onDockPointerUp);
window.addEventListener("pointercancel", stopDockDrag);
window.addEventListener("mousemove", onDockPointerMove);
window.addEventListener("mouseup", onDockPointerUp);
window.addEventListener("pointermove", onResizerPointerMove);
window.addEventListener("pointerup", stopResizerDrag);
window.addEventListener("pointercancel", stopResizerDrag);
window.addEventListener("blur", stopResizerDrag);
window.addEventListener("blur", endResearchFloatingDrag);
window.addEventListener("blur", endResearchFloatingReopenDrag);

panelGrips.forEach((grip) => {
  const panelId = grip.getAttribute("data-panel-grip") || "";
  grip.setAttribute("draggable", "true");
  grip.addEventListener("pointerdown", (event) => handleGripPointerDown(event, panelId, grip));
  grip.addEventListener("mousedown", (event) => handleGripPointerDown(event, panelId, grip));
  grip.addEventListener("dragstart", (event) => {
    startDockDrag(panelId, grip, null);
    if (activeDockDrag) {
      activateDockReorder(panelId);
      activeDockDrag.moved = true;
    }
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = "move";
      event.dataTransfer.setData("text/plain", panelId);
    }
  });
  grip.addEventListener("dragend", () => {
    stopDockDrag();
    suppressNextGripClick = false;
  });
});

document.querySelectorAll(".dock-panel").forEach((panel) => {
  panel.addEventListener("dragover", (event) => {
    if (!activeDockDrag) return;
    event.preventDefault();
    const clientX = event.clientX || panel.getBoundingClientRect().left + panel.getBoundingClientRect().width / 2;
    applyDockReorderForX(activeDockDrag.panelId, clientX);
    highlightDockTarget(clientX);
  });
  panel.addEventListener("drop", (event) => {
    if (!activeDockDrag) return;
    event.preventDefault();
    const clientX = event.clientX || panel.getBoundingClientRect().left + panel.getBoundingClientRect().width / 2;
    applyDockReorderForX(activeDockDrag.panelId, clientX);
    stopDockDrag();
    suppressNextGripClick = false;
  });
});

panelResizers.forEach((resizer) => {
  resizer.addEventListener("pointerdown", (event) => handleResizerPointerDown(event, resizer));
});

activityFlyoutResizer?.addEventListener("pointerdown", (event) => handleResizerPointerDown(event, activityFlyoutResizer));
browserSplitResizer?.addEventListener("pointerdown", (event) => handleResizerPointerDown(event, browserSplitResizer));

langToggle?.addEventListener("click", () => {
  setLanguage(currentLanguage === "zh" ? "en" : "zh");
});

attachButton?.addEventListener("click", () => {
  fileInput?.click();
});

workspaceCodeRunButton?.addEventListener("click", async () => {
  try {
    setActivityPanel("run");
    await loadRunDebugState();
  } catch (error) {
    console.error(error);
    showToast(appErrorMessage(error, "workspace", "toastSendFailed"));
  }
});

fileInput?.addEventListener("change", () => {
  addPendingFiles(fileInput.files);
  fileInput.value = "";
});

const composerDropTarget = document.querySelector(".composer-shell");
let composerDragDepth = 0;
composerDropTarget?.addEventListener("dragenter", (event) => {
  if (!event.dataTransfer?.types?.includes("Files")) return;
  event.preventDefault();
  composerDragDepth += 1;
  composerDropTarget.classList.add("is-file-dragging");
});
composerDropTarget?.addEventListener("dragover", (event) => {
  if (!event.dataTransfer?.types?.includes("Files")) return;
  event.preventDefault();
  event.dataTransfer.dropEffect = "copy";
});
composerDropTarget?.addEventListener("dragleave", (event) => {
  if (!event.dataTransfer?.types?.includes("Files")) return;
  composerDragDepth = Math.max(0, composerDragDepth - 1);
  if (!composerDragDepth) composerDropTarget.classList.remove("is-file-dragging");
});
composerDropTarget?.addEventListener("drop", (event) => {
  if (!event.dataTransfer?.files?.length) return;
  event.preventDefault();
  composerDragDepth = 0;
  composerDropTarget.classList.remove("is-file-dragging");
  addPendingFiles(event.dataTransfer.files);
});

newSessionButton?.addEventListener("click", async () => {
  try {
    await createSession();
  } catch (error) {
    console.error(error);
    showToast(t("toastSendFailed"));
  }
});

settingsToggle?.addEventListener("click", () => {
  if (activeSettingsPanel === "settings-panel") {
    closeSettingsPanels();
    return;
  }
  openSettingsPanel("settings-panel");
  setSettingsTab(activeSettingsTab);
});

activityRailButtons.forEach((button) => {
  button.addEventListener("click", async () => {
    const panel = button.dataset.activityPanel || "nav";
    if (activeActivityPanel === panel) {
      setActivityPanel(null, { preserveMainView: preserveMainViewDuringFlyout() });
      return;
    }
    setActivityPanel(panel);
    if (panel === "git" && !bootstrapData?.git) {
      try {
        await loadGitState(currentGitFetchOptions(currentGitView));
      } catch (error) {
        console.error(error);
        showToast(error?.message || t("gitActionFailed"));
      }
    }
    if (panel === "extensions") {
      try {
        await loadExtensions();
      } catch (error) {
        console.error(error);
        showToast(error?.message || t("toastSendFailed"));
      }
    }
    if (panel === "search") {
      try {
        await loadSearchHealth();
      } catch (error) {
        console.error(error);
        searchState.error = cleanDisplayText(error?.message || "") || appErrorMessage(error, "search", "searchError");
        renderSearchPanel();
        showToast(searchState.error);
      }
    }
    if (panel === "run") {
      try {
        await loadRunDebugState();
      } catch (error) {
        console.error(error);
        showToast(error?.message || t("toastSendFailed"));
      }
    }
  });
});

activityCollapseButtons.forEach((button) => {
  button.addEventListener("click", () => {
    setActivityPanel(null, { preserveMainView: preserveMainViewDuringFlyout() });
  });
});

browserBackButton?.addEventListener("click", async () => {
  const nextIndex = browserState.historyIndex - 1;
  if (nextIndex < 0) return;
  browserState.historyIndex = nextIndex;
  const href = browserState.history[nextIndex] || "";
  if (!href) return;
  try {
    await openUrlInAppBrowser(href, { pushHistory: false });
  } catch (error) {
    console.error(error);
    showToast(cleanDisplayText(error?.message || "") || t("toastSendFailed"));
  }
});

browserRefreshButton?.addEventListener("click", async () => {
  if (!browserState.currentUrl) return;
  try {
    await openUrlInAppBrowser(browserState.currentUrl, { pushHistory: false });
  } catch (error) {
    console.error(error);
    showToast(cleanDisplayText(error?.message || "") || t("toastSendFailed"));
  }
});

browserExternalButton?.addEventListener("click", () => {
  if (!browserState.currentUrl) return;
  window.open(sanitizeHref(browserState.currentUrl), "_blank", "noopener");
});

browserCloseButton?.addEventListener("click", () => {
  closeInAppBrowser();
});

workspaceLauncher?.querySelectorAll("[data-workspace-launch]").forEach((button) => {
  button.addEventListener("click", async () => {
    const action = button.getAttribute("data-workspace-launch") || "";
    if (action === "git") {
      setActivityPanel("git");
      try {
        await loadGitState(currentGitFetchOptions(currentGitView));
      } catch (error) {
        console.error(error);
      }
      return;
    }
    if (action === "terminal") {
      try {
        await createTerminal();
        terminalInput?.focus();
      } catch (error) {
        console.error(error);
      }
      return;
    }
    if (action === "browser") {
      rightSidebarCollapsed = false;
      setMainView("browser");
      return;
    }
    if (action === "files") {
      rightSidebarCollapsed = false;
      preferredDockRightSidebarPanelId = "tree";
      saveDockLayout();
      applyDockLayout();
      return;
    }
    if (action === "side-chat") {
      rightSidebarCollapsed = true;
      saveDockLayout();
      applyDockLayout();
    }
  });
});

leftSidebarToggleButton?.addEventListener("click", () => {
  toggleLeftSidebarVisibility();
});

rightSidebarToggleButton?.addEventListener("click", () => {
  toggleRightSidebarVisibility();
});

browserFrame?.addEventListener("load", () => {
  syncBrowserStateFromFrame({ pushHistory: true });
});

window.addEventListener("message", (event) => {
  const data = event?.data || null;
  if (!data || data.type !== "tokitai-browser-navigate") return;
  const href = sanitizeHref(data.url || "");
  if (!/^https?:\/\//i.test(href)) return;
  openUrlInAppBrowser(href).catch((error) => {
    console.error(error);
    showToast(cleanDisplayText(error?.message || "") || t("toastSendFailed"));
  });
});

terminalRailButton?.addEventListener("click", async () => {
  try {
    await createTerminal();
    terminalInput?.focus();
  } catch (error) {
    console.error(error);
    showToast(appErrorMessage(error, "workspace", "toastSendFailed"));
  }
});

terminalNewInline?.addEventListener("click", async () => {
  try {
    await createTerminal();
    terminalInput?.focus();
  } catch (error) {
    console.error(error);
    showToast(error?.message || t("toastSendFailed"));
  }
});

terminalHideButton?.addEventListener("click", () => {
  terminalDrawerDismissed = true;
  setTerminalDrawerVisible(false);
});

composerStop?.addEventListener("click", async () => {
  const sessionId = String(currentStreamingSessionId || "").trim();
  if (!sessionId) return;
  try {
    await hostClient.chat.stop(sessionId);
  } catch (error) {
    console.error(error);
  } finally {
    endSessionRun(sessionId);
    materializePendingConversationMessages({ sessionId });
    try {
      await refreshBackgroundSessionState();
    } catch (refreshError) {
      console.error(refreshError);
    }
    resetConversationRuntimeState({ preserveInputFocus: true });
  }
});

terminalInput?.addEventListener("keydown", async (event) => {
  if (event.key !== "Enter" || event.shiftKey) return;
  event.preventDefault();
  const value = String(terminalInput.value || "").trim();
  if (!value) return;
  try {
    await sendTerminalInput(value);
    terminalInput.value = "";
  } catch (error) {
    console.error(error);
    showToast(error?.message || t("toastSendFailed"));
  }
});

extensionSearchInput?.addEventListener("input", () => {
  renderExtensionList(extensionSearchInput.value || "");
});

searchModeSwitch?.querySelectorAll(".segment").forEach((button) => {
  button.addEventListener("click", () => {
    searchMode = normalizeChoice(button.dataset.value || "web", ["web", "papers", "models", "datasets", "github"], "web");
    searchState.error = "";
    renderSearchPanel();
    if (searchMode === "datasets" || searchMode === "github" || searchMode === "models") {
      loadSearchHealth().catch((error) => {
        console.error(error);
      });
    }
  });
});

searchRunButton?.addEventListener("click", async () => {
  try {
    await runSearch();
  } catch (error) {
    console.error(error);
    showToast(cleanDisplayText(error?.message || "") || appErrorMessage(error, "search", "searchError"));
  }
});

searchQueryInput?.addEventListener("keydown", async (event) => {
  if (event.key !== "Enter" || event.shiftKey) return;
  event.preventDefault();
  try {
    await runSearch();
  } catch (error) {
    console.error(error);
    showToast(cleanDisplayText(error?.message || "") || appErrorMessage(error, "search", "searchError"));
  }
});

workspacePickerToggle?.addEventListener("click", async () => {
  try {
    await pickWorkspace();
  } catch (error) {
    console.error(error);
    showToast(error?.message || t("toastSendFailed"));
  }
});

workspaceCodeRenderToggle?.addEventListener("click", () => {
  if (!isMarkdownWorkspaceFile(currentWorkspaceFile)) return;
  workspaceCodeRenderMode = workspaceCodeRenderMode === "rendered" ? "source" : "rendered";
  updateWorkspaceCodeView();
});

workspaceCodeSaveButton?.addEventListener("click", async () => {
  try {
    await saveWorkspaceFile();
  } catch (error) {
    console.error(error);
    showToast(error?.message || t("toastSendFailed"));
  }
});

workspaceCodeSearchButton?.addEventListener("click", () => {
  runWorkspaceEditorAction("actions.find");
});

workspaceCodeReplaceButton?.addEventListener("click", () => {
  runWorkspaceEditorAction("editor.action.startFindReplaceAction");
});

workspaceCodeLineButton?.addEventListener("click", () => {
  runWorkspaceEditorAction("editor.action.gotoLine");
});

workspaceCodeSymbolsButton?.addEventListener("click", () => {
  runWorkspaceEditorAction("editor.action.quickOutline");
});

workspaceCodeReferencesButton?.addEventListener("click", () => {
  runWorkspaceFindReferences().catch((error) => {
    console.error(error);
    showToast(error?.message || t("toastSendFailed"));
  });
});

workspaceCodeRenameButton?.addEventListener("click", () => {
  runWorkspaceRenameSymbol().catch((error) => {
    console.error(error);
    showToast(error?.message || t("toastSendFailed"));
  });
});

workspaceCodeSearchButton?.addEventListener("dblclick", () => {
  runWorkspaceEditorAction("actions.findWithSelection");
});

workspaceCodeLineButton?.addEventListener("dblclick", () => {
  runWorkspaceEditorAction("editor.action.revealDefinition");
});

workspaceCodeSymbolsButton?.addEventListener("dblclick", () => {
  runWorkspaceEditorAction("editor.action.peekDefinition");
});

settingsClose?.addEventListener("click", () => {
  closeSettingsPanels();
});

settingsSaveButton?.addEventListener("click", async () => {
  try {
    await saveSettings();
  } catch (error) {
    console.error(error);
    showToast(error?.message || t("toastSendFailed"));
  }
});

modeButtons.forEach((button) => {
  button.addEventListener("click", () => {
    setMainView("chat");
    applyWorkspaceMode(button.dataset.mode || "chat");
  });
});

gitNav?.querySelectorAll("[data-git-view]").forEach((button) => {
  button.addEventListener("click", async () => {
    setActivityPanel("git", { preserveMainView: true });
    setMainView("git");
    setGitView(button.getAttribute("data-git-view") || "overview");
    if (!bootstrapData?.git) {
      try {
        await loadGitState(currentGitFetchOptions(currentGitView));
      } catch (error) {
        console.error(error);
        showToast(error?.message || t("gitActionFailed"));
      }
    }
  });
});

gitRefreshButton?.addEventListener("click", async () => {
  try {
    await runGitAction("refresh");
  } catch (error) {
    console.error(error);
    showToast(error?.message || t("gitActionFailed"));
  }
});

gitFetchButton?.addEventListener("click", async () => {
  try {
    await runGitAction("fetch");
  } catch (error) {
    console.error(error);
    showToast(error?.message || t("gitActionFailed"));
  }
});

gitPullButton?.addEventListener("click", async () => {
  try {
    await runGitAction("pull");
  } catch (error) {
    console.error(error);
    showToast(error?.message || t("gitActionFailed"));
  }
});

gitPushButton?.addEventListener("click", async () => {
  try {
    await runGitAction("push");
  } catch (error) {
    console.error(error);
    showToast(error?.message || t("gitActionFailed"));
  }
});

gitStageAllButton?.addEventListener("click", async () => {
  try {
    await runGitAction("stage_all");
  } catch (error) {
    console.error(error);
    showToast(error?.message || t("gitActionFailed"));
  }
});

gitUnstageAllButton?.addEventListener("click", async () => {
  try {
    await runGitAction("unstage_all");
  } catch (error) {
    console.error(error);
    showToast(error?.message || t("gitActionFailed"));
  }
});

gitCommitButton?.addEventListener("click", async () => {
  const message = window.prompt(t("gitCommitPrompt"), "");
  if (message == null) return;
  if (!message.trim()) return;
  try {
    await runGitAction("commit", { message: message.trim() });
  } catch (error) {
    console.error(error);
    showToast(error?.message || t("gitActionFailed"));
  }
});

sessionMenuRename?.addEventListener("click", async () => {
  const sessionId = activeSessionMenuId;
  if (!sessionId || !bootstrapData?.sessions) return;
  const session = bootstrapData.sessions.find((item) => item.id === sessionId);
  closeSessionMenus();
  const nextTitle = window.prompt(t("renamePrompt"), session?.title || "");
  if (nextTitle == null) return;
  try {
    await renameSession(sessionId, nextTitle.trim() || t("sessionUntitled"));
  } catch (error) {
    console.error(error);
    showToast(t("toastSendFailed"));
  }
});

sessionMenuDelete?.addEventListener("click", async () => {
  const sessionId = activeSessionMenuId;
  if (!sessionId || !bootstrapData?.sessions) return;
  closeSessionMenus();
  if (!window.confirm(t("deleteConfirm"))) return;
  try {
    await deleteSession(sessionId);
  } catch (error) {
    console.error(error);
    showToast(t("toastSendFailed"));
  }
});

messageInput?.addEventListener("keydown", (event) => {
  if (event.key === "Enter" && !event.shiftKey) {
    event.preventDefault();
    sendMessage();
  }
});

applyTranslations();
updateEffortUI();
setActivityPanel("nav", { preserveMainView: true });
applyWorkspaceMode("chat");
setMainView("chat");
setGitView("overview");
setSettingsTab("model");
closeSettingsPanels();
if (effortDisclosure) effortDisclosure.removeAttribute("open");
renderPendingFiles();
hideCodePanel();
renderWorkspaceFile(null);
renderTerminalDrawer();
refreshLatexRendering().catch(() => {});
loadBootstrap().catch((error) => {
  console.error(error);
  showToast(t("toastSendFailed"));
});
window.setInterval(() => {
  const hasRunningSessions = Array.from(sessionRunState.values()).some((state) => Boolean(state?.running));
  if (!hasRunningSessions || document.hidden) return;
  refreshBackgroundSessionState().catch(() => {});
}, 2500);
loadTerminalState()
  .catch(() => {})
  .finally(() => {
    scheduleTerminalPoll(1500);
  });
