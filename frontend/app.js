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
let activeActivityPanel = "nav";
let extensionCatalog = [];
let runDebugState = null;
let terminalState = { sessions: [], active_id: null };
let terminalDrawerDismissed = false;
let terminalPollTimer = null;
let researchFloatingDismissed = false;
let researchFloatingDrag = null;
let researchFloatingReopenDrag = null;
let researchFloatingReopenSuppressClick = false;
let researchFloatingBoardPosition = null;
let preservedMessageScrollTop = null;
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
let workspaceMonacoDefinitionProviders = new Set();
let workspaceMonacoSymbolProviders = new Set();
let workspaceMonacoHoverProviders = new Set();
let workspaceFileTextCache = new Map();
let workspaceSymbolIndexCache = new Map();
let workspaceDiagnosticsTimer = null;
let workspacePendingReveal = null;
let workspaceReferenceMatches = [];

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
let activePanelMenuId = null;
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
let pendingAssistantTextNode = null;
let pendingAssistantStableNode = null;
let pendingAssistantTailNode = null;
let pendingAssistantStatusTextNode = null;
let pendingAssistantStatusTimeNode = null;
let pendingAssistantRenderedStableText = null;
let pendingAssistantRenderedTailText = null;
let pendingAssistantTextFrame = null;
let pendingAssistantStatusFrame = null;
let pendingAssistantBubbleFrame = null;
let pendingBootstrapRefreshPromise = null;
let suppressVisibleStreamBootstrap = false;
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
const toast = document.getElementById("toast");
const sidebarWorkspaceTitle = document.getElementById("sidebar-workspace-title");
const workspaceRootLabel = document.getElementById("workspace-root-label");
const workspaceTitle = document.getElementById("workspace-title");
const riskPill = document.getElementById("risk-pill");
const primaryModel = document.getElementById("primary-model");
const primaryApiUrl = document.getElementById("primary-api-url");
const competitionMode = document.getElementById("competition-mode");
const privacyMode = document.getElementById("privacy-mode");
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
const activityRailButtons = document.querySelectorAll("[data-activity-panel]");
const activityPanels = document.querySelectorAll("[data-activity-panel-id]");
const extensionSearchInput = document.getElementById("extension-search-input");
const extensionList = document.getElementById("extension-list");
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
const dockWorkspace = document.getElementById("dock-workspace");
const panelMenu = document.getElementById("panel-menu");
const panelGrips = document.querySelectorAll("[data-panel-grip]");
const panelResizers = document.querySelectorAll("[data-resizer-after]");
const activityCollapseButtons = document.querySelectorAll("[data-collapse-activity]");
const gitNav = document.getElementById("git-nav");
const gitWorkspace = document.getElementById("git-workspace");
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
const DEFAULT_DOCK_LAYOUT = {
  order: ["sidebar", "chat", "research", "code", "tree"],
  hidden: { sidebar: true, chat: false, research: true, tree: false, code: true },
  widths: { sidebar: 280, chat: 1, research: 360, tree: 280, code: 420 },
};

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

function readDockLayout() {
  try {
    const raw = localStorage.getItem(DOCK_LAYOUT_KEY);
    if (!raw) return structuredClone(DEFAULT_DOCK_LAYOUT);
    const parsed = JSON.parse(raw);
    const widths = { ...DEFAULT_DOCK_LAYOUT.widths, ...(parsed.widths || {}) };
    widths.sidebar = clamp(Number(widths.sidebar || DEFAULT_DOCK_LAYOUT.widths.sidebar), 220, 420);
    widths.tree = clamp(Number(widths.tree || DEFAULT_DOCK_LAYOUT.widths.tree), 200, 2400);
    widths.code = clamp(Number(widths.code || DEFAULT_DOCK_LAYOUT.widths.code), 320, 2400);
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
  const ordered = visiblePanelIds().filter((id) => id !== "code");
  if (!dockLayout.hidden.code) {
    const chatIndex = ordered.indexOf("chat");
    ordered.splice(chatIndex >= 0 ? chatIndex + 1 : ordered.length, 0, "code");
  }
  return ordered;
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
    unsupported: currentLanguage === "zh" ? "当前类型暂不支持预览" : "Preview not supported",
  };

  if (previewKind !== "text" && previewKind !== "markdown") {
    return labelMap[previewKind] || mimeType || (currentLanguage === "zh" ? "文件预览" : "File preview");
  }

  return currentLanguage === "zh"
    ? `${Number(file.line_count || 0)} 行${file.truncated ? " · 预览已截断" : ""}`
    : `${Number(file.line_count || 0)} lines${file.truncated ? " · preview truncated" : ""}`;
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
          ? (currentLanguage === "zh" ? "当前文件" : "Current file")
          : target.path;
        return {
          range: new monaco.Range(position.lineNumber, word.startColumn, position.lineNumber, word.endColumn),
          contents: [
            { value: `**${word.word}**` },
            { value: target.lineText ? `\`${target.lineText}\`` : "" },
            { value: currentLanguage === "zh" ? `定义位置: ${locationText}:${target.lineNumber}` : `Definition: ${locationText}:${target.lineNumber}` },
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
    showToast(currentLanguage === "zh" ? "当前视图不可编辑" : "This view is read only");
  });

  window.requestAnimationFrame(() => {
    workspaceMonacoEditor?.layout?.();
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
    ? `${symbol}：${workspaceReferenceMatches.length} 处引用`
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
    showToast(currentLanguage === "zh" ? "当前轻量版仅支持重命名当前文件中的符号" : "Lightweight rename currently supports the active file only");
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
      ? `已在当前文件重命名 ${localReferences.length} 处`
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
            message: currentLanguage === "zh" ? "括号可能没有正确配对" : "Bracket may be unmatched",
            message: currentLanguage === "zh" ? "括号可能没有正确配对" : "Bracket may be unmatched",
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
      message: currentLanguage === "zh" ? "这里的括号还没有闭合" : "This bracket is not yet closed",
      message: currentLanguage === "zh" ? "这里的括号还没有闭合" : "This bracket is not yet closed",
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
          message: currentLanguage === "zh" ? "这里使用了 Tab 缩进，可能影响格式一致性" : "Tab indentation may cause inconsistent formatting",
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
      <div class="workspace-code-unsupported-title">${escapeHtml(currentLanguage === "zh" ? "当前文件类型暂不支持预览" : "This file type is not supported for preview")}</div>
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
        <div class="workspace-code-unsupported-title">${escapeHtml(currentLanguage === "zh" ? "代码编辑器加载失败" : "Failed to load the code editor")}</div>
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
  return { sidebar: 220, chat: 272, research: 300, tree: 220, code: 320 }[panelId] || 220;
}

function panelCurrentWidth(panelId) {
  const width = Number(dockLayout.widths[panelId] || 0);
  return width > 0 ? width : DEFAULT_DOCK_LAYOUT.widths[panelId] || 280;
}

function panelMaxWidth(panelId) {
  const workspaceWidth = Math.max(dockWorkspace?.clientWidth || 0, window.innerWidth || 0, 1280);
  const chatFloor = panelMinWidth("chat");
  const sharedUpperBound = Math.max(420, workspaceWidth - chatFloor - 24);
  return {
    sidebar: 420,
    research: 460,
    tree: sharedUpperBound,
    code: Math.max(620, sharedUpperBound),
  }[panelId] || sharedUpperBound;
}

function normalizeDockLayout() {
  dockLayout.order = PANEL_IDS.filter((id) => dockLayout.order.includes(id)).concat(
    PANEL_IDS.filter((id) => !dockLayout.order.includes(id)),
  );
  dockLayout.hidden.chat = false;
  dockLayout.hidden.sidebar = true;
  if (currentWorkspaceMode !== "research") {
    dockLayout.hidden.research = true;
  }
  if (dockLayout.hidden.code) {
    dockLayout.order = dockLayout.order.filter((id) => id !== "code").concat("code");
  }
}

function applyDockLayout() {
  if (!dockWorkspace) return;
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
  const resizerTotalWidth = Math.max(0, visible.length - 1) * 10;
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

  visible.forEach((panelId) => {
    const panel = panelElement(panelId);
    if (!panel) return;
    dockWorkspace.appendChild(panel);
    const resizer = resizerByPanelId.get(panelId);
    if (resizer) {
      dockWorkspace.appendChild(resizer);
    }
  });

  hidden.forEach((panelId) => {
    const panel = panelElement(panelId);
    if (!panel) return;
    dockWorkspace.appendChild(panel);
  });

  resizers
    .filter((resizer) => resizer.classList.contains("is-hidden"))
    .forEach((resizer) => {
    dockWorkspace.appendChild(resizer);
  });
}

function closePanelMenu() {
  activePanelMenuId = null;
  if (panelMenu) {
    panelMenu.hidden = true;
    panelMenu.innerHTML = "";
  }
  panelGrips.forEach((grip) => grip.setAttribute("aria-expanded", "false"));
}

function togglePanelHidden(panelId) {
  if (panelId === "chat") return;
  dockLayout.hidden[panelId] = !dockLayout.hidden[panelId];
  saveDockLayout();
  applyDockLayout();
  closePanelMenu();
}

function openPanelMenu(panelId, anchor) {
  if (!panelMenu || !anchor) return;
  if (activePanelMenuId === panelId && !panelMenu.hidden) {
    closePanelMenu();
    return;
  }

  activePanelMenuId = panelId;
  const hidden = Boolean(dockLayout.hidden[panelId]);
  const hiddenCandidates = PANEL_IDS.filter(
    (id) => id !== "chat" && dockLayout.hidden[id] && id !== panelId,
  );
  panelMenu.innerHTML = `
    ${
      panelId !== "chat"
        ? `<button class="panel-floating-menu-item" type="button" data-panel-toggle="${escapeHtml(panelId)}">${hidden ? "Show panel" : "Hide panel"}</button>`
        : ""
    }
    ${
      hiddenCandidates.length
        ? `<div class="panel-floating-menu-label">Hidden Panels</div>${hiddenCandidates
            .map(
              (id) =>
                `<button class="panel-floating-menu-item" type="button" data-panel-show="${escapeHtml(id)}">Show ${escapeHtml(id)}</button>`,
            )
            .join("")}`
        : ""
    }
  `;
  panelMenu.hidden = false;
  const rect = anchor.getBoundingClientRect();
  const menuWidth = 168;
  const left = Math.min(window.innerWidth - menuWidth - 12, Math.max(12, rect.left + rect.width / 2 - menuWidth / 2));
  panelMenu.style.left = `${left}px`;
  panelMenu.style.top = `${rect.bottom + 8}px`;
  panelGrips.forEach((grip) => {
    grip.setAttribute("aria-expanded", grip === anchor ? "true" : "false");
  });
  panelMenu.querySelectorAll("[data-panel-toggle]").forEach((button) => {
    button.addEventListener("click", () => togglePanelHidden(button.getAttribute("data-panel-toggle") || ""));
  });
  panelMenu.querySelectorAll("[data-panel-show]").forEach((button) => {
    button.addEventListener("click", () => {
      const targetId = button.getAttribute("data-panel-show") || "";
      if (!targetId) return;
      dockLayout.hidden[targetId] = false;
      saveDockLayout();
      applyDockLayout();
      closePanelMenu();
    });
  });
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
  dockLayout.hidden.code = false;
  const next = dockLayout.order.filter((id) => id !== "code");
  const chatIndex = next.indexOf("chat");
  next.splice(chatIndex >= 0 ? chatIndex + 1 : next.length, 0, "code");
  dockLayout.order = next;
  saveDockLayout();
  applyDockLayout();
  requestAnimationFrame(() => restoreMessageScrollPosition());
}

function hideCodePanel() {
  captureMessageScrollPosition();
  dockLayout.hidden.code = true;
  isWorkspaceCodeOpen = false;
  activeWorkspaceFilePath = null;
  currentWorkspaceFile = null;
  workspacePendingReveal = null;
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
  const afterId = handle?.getAttribute("data-resizer-after") || "";
  const visible = renderedPanelIds();
  const leftIndex = visible.indexOf(afterId);
  const leftPanelId = leftIndex >= 0 ? visible[leftIndex] : "";
  const rightPanelId = leftIndex >= 0 ? visible[leftIndex + 1] || "" : "";
  if (!leftPanelId || !rightPanelId) return;
  activeResizerDrag = {
    handle,
    pointerId: event.pointerId ?? null,
    afterId,
    leftPanelId,
    rightPanelId,
    startX: event.clientX,
  };
  handle.classList.add("is-active");
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
    dockLayout.hidden.research = true;
  }
  modeButtons.forEach((button) => {
    const active = (button.dataset.mode || "chat") === currentWorkspaceMode;
    button.classList.toggle("is-active", active);
    button.setAttribute("aria-selected", active ? "true" : "false");
  });
  if (messageInput) {
    messageInput.placeholder = currentWorkspaceMode === "research"
      ? (currentLanguage === "zh"
          ? "Agent 默认做轻量实现；输入 /spec 开头可强制进入科研流程，Enter 发送。"
          : "Agent defaults to lightweight implementation. Start with /spec to force a research workflow. Press Enter to send.")
      : t("composerPlaceholder");
  }
  if (composerStop) {
    composerStop.textContent = currentWorkspaceMode === "research"
      ? (currentLanguage === "zh" ? "中止研究" : "Stop research")
      : "Stop";
  }
  if (activityLabel) {
    if (currentWorkspaceMode === "research" && activityLabel === t("activityReviewing")) {
      setActivity(currentLanguage === "zh" ? "正在研究" : "Researching");
    } else if (currentWorkspaceMode === "chat" && (
      activityLabel === "姝ｅ湪鐮旂┒" || activityLabel === "Researching"
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
  dockLayout.hidden.research = !researchDetailOpen;
  if (researchDetailOpen) {
    researchFloatingDismissed = false;
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
  return `${clean.slice(0, Math.max(0, maxLength - 1)).trim()}…`;
}

function looksLikeCorruptedText(value) {
  const text = String(value || "").trim();
  if (!text) return false;
  const countChars = (chars) => [...text].filter((char) => chars.includes(char)).length;
  const replacementCount = countChars(["锟", "絔"]);
  const questionCount = countChars(["?", "锛", "焆"]);
  const mojibakePunctuation = countChars(["閳", "ラ", "妴", "閿", "涢", "敓"]);
  const mojibakeMarkers = countChars(["閸", "欏", "", "缂", "侀", "弬", "瀵", "伴", "梼", "闁", "玗"]);
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
    const match = decoded.match(new RegExp(`"${name}":"([^"]*)"`, "i"));
    return match ? cleanDisplayText(match[1]) : "";
  };
  const prefixMatch = decoded.match(/^([^"{]+失败)[：:]/);
  const prefix = cleanDisplayText(prefixMatch ? prefixMatch[1] : "");
  const operation = readField("operation");
  const path = readField("path");
  const suggestion = readField("suggestion");
  const message = readField("message");
  const target = displayFileNameOnly(path);

  if (message || suggestion || operation) {
    const operationLabel = cleanDisplayText(prefix || operation);
    if (/系统找不到指定的路径|os error 3/i.test(message || decoded)) {
      return currentLanguage === "zh"
        ? `${operationLabel || "路径检查"}：目标目录${target ? ` ${target}` : ""} 还没准备好，我会先把路径修正后再继续。`
        : `${operationLabel || "Path check"}: the target path${target ? ` ${target}` : ""} is not ready yet, so I’m correcting it before continuing.`;
    }
    if (/无效的编辑模式/i.test(message || decoded)) {
      return currentLanguage === "zh"
        ? `${operationLabel || "文件编辑"}：刚才用了不支持的编辑模式，我正在换成正确方式继续。`
        : `${operationLabel || "File edit"}: that edit mode was not supported, so I’m switching to a valid one and continuing.`;
    }
    if (/未找到要替换的文本/i.test(message || decoded)) {
      return currentLanguage === "zh"
        ? `${operationLabel || "文件编辑"}：原始文本没有精确匹配上，我先重新定位再继续修改。`
        : `${operationLabel || "File edit"}: the source text did not match exactly, so I’m re-locating it before editing again.`;
    }
    const parts = [
      operationLabel,
      cleanDisplayText(message),
      cleanDisplayText(suggestion),
    ].filter(Boolean);
    if (parts.length) {
      return parts.join(currentLanguage === "zh" ? "：": ": ");
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
  return !/```|^\s*[-*]\s+/m.test(source);
}

function splitNarrationClauses(text) {
  return String(text || "")
    .replace(/\r\n/g, "\n")
    .split(/\n+/)
    .flatMap((line) => line
      .split(/(?<=[。！？!?])/)
      .map((item) => item.trim())
      .filter(Boolean))
    .map((raw) => ({ raw, normalized: normalizeNarrationClause(raw) }))
    .filter((item) => item.normalized);
}

function normalizeNarrationClause(text) {
  return sanitizeMessageContent(String(text || ""))
    .toLowerCase()
    .replace(/^#+\s*/g, "")
    .replace(/^(好(?:的)?|然后|接下来|现在|我先|我会|让我|目前)[，。:\s]*/u, "")
    .replace(/(所有|全部)/g, "")
    .replace(/(齐备|齐全|就绪|准备好了)/g, "就绪")
    .replace(/(工作区状态和依赖情况|当前工作区状态和依赖情况|工作区和依赖情况)/g, "工作区依赖")
    .replace(/(开始创建工程文件|现在创建工程文件|开始创建目录和脚本|现在创建目录和脚本)/g, "创建工程文件")
    .replace(/(开始创建并运行脚本|现在创建并运行脚本)/g, "创建并运行脚本")
    .replace(/[`*_#]/g, " ")
    .replace(/[，。！？!:\s]+/g, "")
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
    ? "历史文本编码异常，已省略。"
    : "Legacy text omitted due to corrupted encoding.";
}

function isLowValueSummaryText(value) {
  const text = sanitizeMessageContent(String(value || "")).trim();
  if (!text || looksLikeCorruptedText(text)) return true;
  const markers = [
    "鏃犳硶鐞嗚В鎮ㄥ彂閫佺殑鍐呭",
    "无法正常显示的内容",
    "您的输入仍然显示为无法识别的字符",
    "涔辩爜瀛楃",
    "请重新描述您的需求",
    "重新发送一条",
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
    return `<a href="${escapeHtml(safeHref)}" target="_blank" rel="noreferrer">${label}</a>`;
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
  const isHeadingLine = (line) => /^(#{1,6})\s+/.test(line);
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
      const [, hashes, content] = line.match(/^(#{1,6})\s+(.*)$/) || [];
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
      zh ? "会话未就绪，请稍后重试。" : "Session is not ready yet. Please try again."
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
      zh ? "模型流式连接失败，请检查 API URL、API Key、模型名称与流式支持。" : "Model streaming connection failed. Check API URL, API key, model name, and streaming support."
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
      zh ? "SSE 中断，请重试本轮或检查流式连接。" : "SSE stream was interrupted. Retry the turn or check the streaming connection."
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
      zh ? "工具执行失败，请查看当前工具输出后重试。" : "Tool execution failed. Review the tool output and try again."
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
      zh ? "发送失败，请稍后重试。" : "Send failed. Please try again."
    );
  }

  return result(
    "generic",
    zh ? "出现了一个问题，请稍后再试。" : "Something went wrong. Please try again."
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
    return currentLanguage === "zh" ? "缺少运行依赖。" : "Missing runtime dependency.";
  }
  const prefix = currentLanguage === "zh" ? "缺少可执行文件：" : "Missing executable:";
  return `${prefix} ${items.join(", ")}`;
}

function parseAgentInputProtocol(rawContent) {
  const content = String(rawContent || "");
  const trimmed = content.trim();
  const inAgentMode = currentWorkspaceMode === "research";
  if (!inAgentMode) {
    return {
      outbound: trimmed,
      display: trimmed,
      mode: "chat",
      forceResearch: false,
    };
  }

  if (/^\/spec(?:\s|$)/i.test(trimmed)) {
    const stripped = trimmed.replace(/^\/spec(?:\s+)?/i, "").trim();
    return {
      outbound: stripped,
      display: stripped,
      mode: "research",
      forceResearch: true,
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
  messageStream.scrollTop = messageStream.scrollHeight;
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
  const needsBreak =
    !/[\\n\\s]$/.test(left) &&
    !/^[\\n\\s]/.test(uniqueRight);
  return needsBreak ? `${left}\n\n${uniqueRight}` : `${left}${uniqueRight}`;
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
  const rawLabel = String(event?.label || event?.detail || "Running").trim();
  const labelMapZh = {
    starting: "正在准备",
    planning: "正在规划",
    execution: "正在思考",
    delegation: "正在调用工具",
    review: "正在审查",
    verifier: "正在验证",
    subagent: "子代理处理中",
    permission_required: "等待批准",
    editing: "正在编辑",
    tool_complete: "工具执行完成",
  };
  const labelMapEn = {
    starting: "Preparing",
    planning: "Planning",
  };
  const detailMapZh = {
    "Main agent is executing the current step": "主代理正在执行当前步骤",
    "Dispatching tool work": "正在分派工具执行",
    "Reviewer subagent is checking the turn": "审查子代理正在检查本轮结果",
  };
  const label = currentLanguage === "zh"
    ? (labelMapZh[rawLabel] || rawLabel)
    : (labelMapEn[rawLabel] || rawLabel);
  const rawDetail = String(event?.detail || "").trim();
  const detail = currentLanguage === "zh"
    ? (detailMapZh[rawDetail] || rawDetail)
    : rawDetail;
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
    running: currentLanguage === "zh" ? "进行中" : "Running",
    pass: currentLanguage === "zh" ? "通过" : "Pass",
    complete: currentLanguage === "zh" ? "完成" : "Complete",
    repair: currentLanguage === "zh" ? "待修复" : "Repair",
    failed: currentLanguage === "zh" ? "失败" : "Failed",
  };
  return labels[normalized] || status || (currentLanguage === "zh" ? "进行中" : "Running");
}

function renderAgentName(name) {
  const normalized = String(name || "").trim().toLowerCase();
  const labels = {
    main: currentLanguage === "zh" ? "主代理" : "Main",
    planner: currentLanguage === "zh" ? "规划器" : "Planner",
    reviewer: currentLanguage === "zh" ? "审查器" : "Reviewer",
    verifier: currentLanguage === "zh" ? "验证器" : "Verifier",
    repairer: currentLanguage === "zh" ? "修复器" : "Repairer",
    critic: currentLanguage === "zh" ? "批判审查" : "Critic",
    researcher: currentLanguage === "zh" ? "研究审查" : "Researcher",
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
    .replace(/[`"'()[\]{}:;,，。！？!?.、\-_/\\|]+/g, " ")
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
    timestamp: Date.now(),
  });
  activeAssistantTurn.worklog = items.slice(-6);
}

function pushAssistantProgressWorklogText(text) {
  const cleanText = cleanDisplayText(text || "");
  if (!cleanText) return;
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
}

function ensurePendingAssistantBubbleForRuntime() {
  if (!activeAssistantTurn || pendingAssistantBubble) return;
  const currentSessionId = String(currentStreamingSessionId || bootstrapData?.current_session_id || "").trim();
  if (!getSessionRunState(currentSessionId)?.running) return;
  appendAssistantBubble(activeAssistantTurn.text || "");
}

function describeActivityWorklog(event) {
  const label = String(event?.label || "").trim();
  const detail = cleanDisplayText(event?.detail || "");
  const phase = String(event?.phase || "").trim().toLowerCase();
  const agent = renderAgentName(event?.agent);
  const meta = cleanDisplayText(event?.meta || "");

  if (label === "execution") {
    return {
      kind: "execution",
      text: currentLanguage === "zh"
        ? `我开始处理当前步骤了。`
        : `I’m starting the current step.`,
      dedupeKey: `activity:${label}:${detail || phase || "execution"}`,
    };
  }
  if (label === "delegation") {
    return {
      kind: "delegation",
      text: currentLanguage === "zh"
        ? `我正在分派工具执行${meta ? `：${meta}` : "。"}`
        : `I’m dispatching tool work${meta ? `: ${meta}` : "."}`,
      dedupeKey: `activity:${label}:${meta || detail || ""}`,
    };
  }
  if (label === "review") {
    return {
      kind: "review",
      text: currentLanguage === "zh"
        ? `我先做一轮审查。`
        : `I’m running a review pass.`,
      dedupeKey: `activity:${label}:${detail || meta || ""}`,
    };
  }
  if (label === "repair") {
    return {
      kind: "repair",
      text: currentLanguage === "zh"
        ? `我发现还有缺口，正在补修后再验证。`
        : `I found a gap, so I’m repairing it before verifying again.`,
      dedupeKey: `activity:${label}:${detail || meta || ""}`,
    };
  }
  if (label === "permission_required") {
    return {
      kind: "approval",
      text: currentLanguage === "zh"
        ? `这里需要你的批准，我先停在这一步。`
        : `I need your approval here, so I’m pausing at this step.`,
      dedupeKey: "activity:permission_required",
    };
  }
  if (phase === "finalize") {
    return {
      kind: "finalize",
      text: currentLanguage === "zh"
        ? `我在做最后收尾，把结果整理进当前会话。`
        : `I’m wrapping things up and persisting the result into this session.`,
      dedupeKey: `activity:finalize:${detail || label}`,
    };
  }
  if (!label && !detail) return null;
  return {
    kind: phase || label || "activity",
    text: currentLanguage === "zh"
      ? `${agent || "我"}正在推进这一步。`
      : `${agent || "I’m"} moving this step forward.`,
    dedupeKey: `activity:${label}:${detail}:${meta}`,
  };
}

function describeToolWorklog(tool) {
  if (!tool) return null;
  const name = cleanDisplayText(tool.name || "", currentLanguage === "zh" ? "工具" : "tool");
  const status = String(tool.status || "").trim().toLowerCase();
  const fileName = displayFileNameOnly(tool.file_path || "");
  if (status === "pending" || status === "running") {
    return {
      kind: "tool",
      text: currentLanguage === "zh"
        ? `我正在调用 ${name}${fileName ? `，目标是 ${fileName}` : ""}。`
        : `I’m calling ${name}${fileName ? ` for ${fileName}` : ""}.`,
      dedupeKey: `tool:${tool.call_id || name}:running:${fileName}`,
    };
  }
  if (status === "complete") {
    return {
      kind: "tool",
      text: currentLanguage === "zh"
        ? `${name} 已经跑完了，我继续往下处理。`
        : `${name} finished, and I’m continuing from there.`,
      dedupeKey: `tool:${tool.call_id || name}:complete`,
    };
  }
  if (status === "failed") {
    return {
      kind: "tool",
      text: currentLanguage === "zh"
        ? `${name} 这一步出错了，我会根据结果调整下一步。`
        : `${name} failed here, so I’ll adjust the next step based on that result.`,
      dedupeKey: `tool:${tool.call_id || name}:failed`,
    };
  }
  return null;
}

function describeEditedFileWorklog(file) {
  if (!file?.path) return null;
  const fileName = displayFileNameOnly(file.path);
  return {
    kind: "edit",
    text: currentLanguage === "zh"
      ? `我开始修改 ${fileName}，当前变更是 +${Number(file.added || 0)} / -${Number(file.removed || 0)}。`
      : `I’m updating ${fileName}, currently at +${Number(file.added || 0)} / -${Number(file.removed || 0)}.`,
    dedupeKey: `edit:${file.path}:${Number(file.added || 0)}:${Number(file.removed || 0)}`,
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
      text: currentLanguage === "zh"
        ? `${name} 已经开始处理它负责的部分了。`
        : `${name} has started on its part of the task.`,
      dedupeKey: `subagent:${subagent.id || name}:running`,
    };
  }
  if (status === "complete" || status === "pass") {
    return {
      kind: "subagent",
      text: currentLanguage === "zh"
        ? `${name} 已经返回结果了${output ? `：${output}` : "。"}`
        : `${name} returned a result${output ? `: ${output}` : "."}`,
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
      text: currentLanguage === "zh"
        ? "我正在做硬验证，确认文件、输出和实验结果都对得上。"
        : "I’m running hard verification to make sure files, outputs, and experiment results all line up.",
      dedupeKey: "verifier:running",
    };
  }
  if (status === "pass" || status === "complete") {
    return {
      kind: "verifier",
      text: currentLanguage === "zh"
        ? `验证已经通过了${summary ? `：${summary}` : "。"}`
        : `Verification passed${summary ? `: ${summary}` : "."}`,
      dedupeKey: `verifier:${status}:${summary}`,
    };
  }
  if (status === "repair" || status === "failed") {
    return {
      kind: "verifier",
      text: currentLanguage === "zh"
        ? `验证发现了问题${summary ? `：${summary}` : "，我继续修。"}`
        : `Verification found an issue${summary ? `: ${summary}` : ", so I’m continuing with a repair pass."}`,
      dedupeKey: `verifier:${status}:${summary}`,
    };
  }
  return null;
}

function describeCompletionWorklog(event) {
  const detail = cleanDisplayText(event?.activity?.detail || "");
  if (!detail) return null;
  if (/resumable checkpoint|safe checkpoint/i.test(detail)) {
    return {
      kind: "checkpoint",
      text: currentLanguage === "zh"
        ? "我先停在一个可继续的检查点，当前工作区里的内容已经保留下来了。"
        : "I’m pausing at a resumable checkpoint, and the current workspace state has been preserved.",
      dedupeKey: "complete:checkpoint",
    };
  }
  if (/verification did not pass/i.test(detail)) {
    return {
      kind: "verifier",
      text: currentLanguage === "zh"
        ? "这一轮先停下来了，因为验证还没有通过。"
        : "This turn stopped here because verification has not passed yet.",
      dedupeKey: "complete:verification_failed",
    };
  }
  return null;
}

function renderAgentRuntimeStrip() {
  if (!agentRuntimeStrip) return;
  const file = Array.isArray(liveEditedFiles) && liveEditedFiles.length
    ? liveEditedFiles[liveEditedFiles.length - 1]
    : null;
  const currentSessionId = String(currentStreamingSessionId || bootstrapData?.current_session_id || "").trim();
  const isRunning = Boolean(getSessionRunState(currentSessionId)?.running);
  const hasLiveDiffs = Boolean(activeAssistantTurn?.diffs?.length);

  if ((!file && !hasLiveDiffs) || (!isRunning && !file)) {
    agentRuntimeStrip.hidden = true;
    agentRuntimeStrip.innerHTML = "";
    return;
  }

  const displayFile = file || activeAssistantTurn?.diffs?.[activeAssistantTurn.diffs.length - 1] || null;
  if (!displayFile) {
    agentRuntimeStrip.hidden = true;
    agentRuntimeStrip.innerHTML = "";
    return;
  }

  if (activeAssistantTurn && file) {
    upsertDiffEntry(file);
  }

  agentRuntimeStrip.hidden = false;
  agentRuntimeStrip.innerHTML = `
    <div class="agent-runtime-chip-wrap">
      <button
        class="agent-runtime-chip agent-runtime-chip-action"
        type="button"
        data-open-workspace-file="${escapeHtml(displayFile.path || "")}"
        data-open-workspace-line="1"
        data-open-workspace-column="1"
      >
        <span class="agent-runtime-label">${escapeHtml(currentLanguage === "zh" ? "编辑中" : "Editing")}</span>
        <div class="agent-runtime-value">
          <span class="agent-runtime-path">${escapeHtml(displayFileNameOnly(displayFile.path || ""))}</span>
          <span class="agent-runtime-stats">+${escapeHtml(String(displayFile.added || 0))} / -${escapeHtml(String(displayFile.removed || 0))}</span>
        </div>
      </button>
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
        <div class="permission-title">${escapeHtml(currentLanguage === "zh" ? "等待工具批准" : "Awaiting tool approval")}</div>
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

  if (!preserveMainView) {
    if (nextPanel === "git") {
      setMainView("git");
    } else if (currentMainView === "git" && nextPanel !== "git") {
      setMainView("chat");
    }
  }
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
              <span class="extension-card-desc">${escapeHtml(config.category || "")}${config.file_hint ? ` 路 ${escapeHtml(config.file_hint)}` : ""}</span>
              <span class="run-card-task">${escapeHtml(config.task_type || "launch")} 路 ${escapeHtml(config.detail || "")}</span>
            </button>
          `,
        )
        .join("")
    : `<div class="git-empty">${escapeHtml(currentLanguage === "zh" ? "当前项目没有检测到可运行配置。" : "No runnable configuration detected.")}</div>`;

  runDebugSession.hidden = !active;
  runDebugSession.innerHTML = active
    ? `
      <div class="run-session-head">
        <strong>${escapeHtml(active.title || "")}</strong>
        <button class="git-inline-action is-danger" type="button" data-run-debug-stop="true">${escapeHtml(currentLanguage === "zh" ? "停止" : "Stop")}</button>
      </div>
      <div class="run-session-meta">PID ${escapeHtml(String(active.pid || ""))} 路 ${escapeHtml(active.started_at || "")}</div>
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
    : `<div class="git-empty">${escapeHtml(currentLanguage === "zh" ? "当前项目没有检测到可运行配置。" : "No runnable configuration detected.")}</div>`;

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
  if (!pendingFiles.length) {
    composerAttachments.hidden = true;
    composerAttachments.innerHTML = "";
    return;
  }

  composerAttachments.hidden = false;
  composerAttachments.innerHTML = "";
  pendingFiles.forEach((file, index) => {
    const chip = document.createElement("div");
    chip.className = "attachment-chip";
    chip.innerHTML = `
      <span>${escapeHtml(file.path || file.name || `file-${index + 1}`)}</span>
      <button class="attachment-remove" type="button" aria-label="Remove">x</button>
    `;
    chip.querySelector(".attachment-remove")?.addEventListener("click", () => {
      pendingFiles = pendingFiles.filter((_item, fileIndex) => fileIndex !== index);
      renderPendingFiles();
    });
    composerAttachments.appendChild(chip);
  });
}

function sanitizeMessageContent(text) {
  const raw = String(text || "");
  const dsmlStarts = [
    "<||DSML",
    "</||DSML",
    "||DSML||",
    "<锝滐綔DSML",
    "</锝滐綔DSML",
    "锝滐綔DSML锝滐綔",
    "<DSML",
    "</DSML",
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
  ];
  for (const marker of toolNarrationMarkers) {
    const index = raw.indexOf(marker);
    if (index !== -1 && (cutIndex === -1 || index < cutIndex)) {
      cutIndex = index;
    }
  }
  for (const marker of ["Tool", "Arguments", "Result summary", "{\"operation\""]) {
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
    thinking: [],
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
    receivedDelta: false,
  };
}

function cloneAssistantTurnState(turn) {
  if (!turn) return null;
  return {
    ...turn,
    thinking: Array.isArray(turn.thinking) ? turn.thinking.map((item) => ({ ...item })) : [],
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
    text: mergeAssistantText(base.text, live.text, options),
    thinking: richerAssistantCollection(base.thinking, live.thinking),
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
    receivedDelta: Boolean(base.receivedDelta || live.receivedDelta),
  };
}

function shouldSuppressInlineAssistantCode(text, diffs = []) {
  const content = String(text || "");
  const isAgentLikeMode = currentWorkspaceMode === "research" || currentWorkspaceMode === "agent";
  const looksCodeLike = /```|(?:^|\n)\s*(?:def |class |function |import |from |const |let |var |pub |fn |use |#include )/m.test(content);
  const codeFenceCount = (content.match(/```/g) || []).length;
  const codeyLineCount = content
    .split(/\r?\n/)
    .filter((line) =>
      /^\s*(?:def |class |function |import |from |const |let |var |pub |fn |use |#include )/.test(line),
    )
    .length;
  const longCodeLike = looksCodeLike && (
    content.trim().length > 420
      || codeFenceCount >= 2
      || codeyLineCount >= 8
  );
  const hasDiffs = Array.isArray(diffs) && diffs.length > 0;
  return Boolean(
    (isAgentLikeMode || hasDiffs) && longCodeLike,
  );
}

function visibleAssistantWorkspaceNotice() {
  return currentLanguage === "zh"
    ? "本轮实现已直接写入工作区文件，详细代码不在对话区展开。"
    : "This turn was written directly into workspace files. Full source is not expanded in chat.";
}

function zhLabel(zh, en) {
  return currentLanguage === "zh" ? zh : en;
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
  return currentLanguage === "zh" ? "待审阅" : "Pending review";
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
  const preservedRunningSessionId = String(options.preserveRunningSessionId || "").trim();
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
  (Array.isArray(snapshot.progress_updates) ? snapshot.progress_updates : []).forEach((entry) => {
    pushAssistantProgressWorklogText(entry);
  });
  activeAssistantTurn.subagents = Array.isArray(snapshot.subagents)
    ? dedupeSubagentEntries(snapshot.subagents)
    : [];
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
  if (
    getSessionRunState(currentSessionId)?.running &&
    (String(activeAssistantTurn.text || "").trim() ||
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
}

function captureMessageScrollPosition() {
  if (!messageStream) return;
  preservedMessageScrollTop = messageStream.scrollTop;
}

function restoreMessageScrollPosition() {
  if (!messageStream || preservedMessageScrollTop == null) return;
  messageStream.scrollTop = preservedMessageScrollTop;
  preservedMessageScrollTop = null;
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
  return snapshot;
}

function visibleConversationMessages(messages) {
  return (Array.isArray(messages) ? messages : []).filter(
    (message) =>
      message &&
      (message.kind === "message" ||
        message.kind === "thinking" ||
        message.kind === "diff" ||
        message.kind === "subagent" ||
        message.kind === "verification"),
  );
}

function persistConversationMessages(messages, { sessionId = null } = {}) {
  const visibleMessages = visibleConversationMessages(messages);
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
    ? `本轮执行已中断：${classified.message || "发送失败"}`
    : `This turn stopped early: ${classified.message || "Send failed."}`;
  nextMessages.push({
    kind: "message",
    role: "assistant",
    content: failureText,
  });
  persistConversationMessages(nextMessages, { sessionId: currentStreamingSessionId });
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
  pendingAssistantTextNode = scope?.querySelector("[data-streaming-markdown]") || null;
  pendingAssistantStableNode = scope?.querySelector("[data-streaming-stable]") || null;
  pendingAssistantTailNode = scope?.querySelector("[data-streaming-tail]") || null;
  pendingAssistantStatusTextNode = scope?.querySelector("[data-turn-activity]") || null;
  pendingAssistantStatusTimeNode = scope?.querySelector("[data-turn-elapsed]") || null;
  pendingAssistantRenderedStableText = null;
  pendingAssistantRenderedTailText = null;
}

function findStreamingRenderableBoundary(text) {
  const source = String(text || "");
  if (!source) return 0;
  return source.length;
}

function renderStreamingAssistantContent(content, options = {}) {
  const text = String(content || "");
  const placeholder = options.placeholder || pendingAssistantPlaceholderText();
  if (!text) {
    return {
      stableText: "",
      stableHtml: "",
      tailText: "",
      tailHtml: escapeHtml(placeholder),
      isEmpty: true,
      tailVisible: true,
    };
  }

  const boundary = findStreamingRenderableBoundary(text);
  const stableText = text.slice(0, boundary).trimEnd();
  const tailText = text.slice(boundary);
  const tailHtml = tailText
    ? tailText
        .replace(/\r\n/g, "\n")
        .split("\n")
        .map((line) => renderInlineMarkdown(line))
        .join("<br>")
    : "";
  return {
    stableText,
    stableHtml: stableText ? renderMarkdown(stableText) : "",
    tailText,
    tailHtml,
    isEmpty: false,
    tailVisible: Boolean(tailText),
  };
}

function resetPendingAssistantRenderState() {
  pendingAssistantTextNode = null;
  pendingAssistantStableNode = null;
  pendingAssistantTailNode = null;
  pendingAssistantStatusTextNode = null;
  pendingAssistantStatusTimeNode = null;
  pendingAssistantRenderedStableText = null;
  pendingAssistantRenderedTailText = null;
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
  const worklog = Array.isArray(turn.worklog) ? turn.worklog : [];
  if (worklog.length) {
    const latest = worklog[worklog.length - 1] || null;
    const previous = worklog.length > 1 ? worklog[worklog.length - 2] : null;
    const parts = [previous?.text, latest?.text]
      .map((item) => cleanDisplayText(String(item || "").trim()))
      .filter(Boolean);
    return [...new Set(parts)].join(currentLanguage === "zh" ? "\n\n" : "\n\n");
  }
  const latestProcess = Array.isArray(turn.process) && turn.process.length
    ? turn.process[turn.process.length - 1]
    : null;
  if (latestProcess?.detail) {
    return cleanDisplayText(String(latestProcess.detail || "").trim());
  }
  if (latestProcess?.label) {
    return cleanDisplayText(String(latestProcess.label || "").trim());
  }
  return "";
}

function syncPendingAssistantText() {
  if (!pendingAssistantTextNode) return;
  const explicitContent = String(activeAssistantTurn?.text || "");
  const content = explicitContent.trim() ? explicitContent : "";
  const shouldSuppressInlineCode = shouldSuppressInlineAssistantCode(
    content,
    activeAssistantTurn?.diffs || [],
  );
  const visibleContent = shouldSuppressInlineCode
    ? visibleAssistantWorkspaceNotice()
    : content;
  const parts = renderStreamingAssistantContent(visibleContent, {
    placeholder: pendingAssistantPlaceholderText(),
  });
  if (!pendingAssistantStableNode || !pendingAssistantTailNode) {
    pendingAssistantTextNode.innerHTML = visibleContent
      ? renderMarkdown(visibleContent)
      : escapeHtml(pendingAssistantPlaceholderText());
  } else {
    if (pendingAssistantRenderedStableText !== parts.stableText) {
      pendingAssistantStableNode.innerHTML = parts.stableHtml;
      pendingAssistantRenderedStableText = parts.stableText;
    }
    if (pendingAssistantRenderedTailText !== parts.tailText) {
      pendingAssistantTailNode.innerHTML = parts.tailHtml;
      pendingAssistantRenderedTailText = parts.tailText;
    }
    pendingAssistantTailNode.hidden = !parts.tailVisible;
    pendingAssistantStableNode.hidden = !parts.stableText;
  }
  pendingAssistantTextNode.parentElement?.classList.toggle("codex-answer-empty", parts.isEmpty);
}

function schedulePendingAssistantTextSync() {
  if (pendingAssistantTextFrame != null) return;
  pendingAssistantTextFrame = window.requestAnimationFrame(() => {
    pendingAssistantTextFrame = null;
    syncPendingAssistantText();
    syncPendingAssistantStatus();
    scrollMessageStreamToBottom();
  });
}

function schedulePendingAssistantStatusSync() {
  if (pendingAssistantStatusFrame != null) return;
  pendingAssistantStatusFrame = window.requestAnimationFrame(() => {
    pendingAssistantStatusFrame = null;
    if (!pendingAssistantBubble || !activeAssistantTurn) return;
    syncPendingAssistantStatus();
    scrollMessageStreamToBottom();
  });
}

function refreshPendingAssistantBubble() {
  if (!pendingAssistantBubble || !activeAssistantTurn) return;
  if (pendingAssistantBubbleFrame != null) return;
  pendingAssistantBubbleFrame = window.requestAnimationFrame(() => {
    pendingAssistantBubbleFrame = null;
    if (!pendingAssistantBubble || !activeAssistantTurn) return;
    const keepBottom = isNearMessageStreamBottom();
    pendingAssistantBubble.innerHTML = renderAssistantTurn(activeAssistantTurn, { streaming: true });
    bindTurnInteractionHandlers(pendingAssistantBubble);
    bindPendingAssistantNodes(pendingAssistantBubble);
    syncPendingAssistantText();
    syncPendingAssistantStatus();
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
    executing: currentLanguage === "zh" ? "进行中" : "Running",
    complete: currentLanguage === "zh" ? "已完成" : "Done",
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
    pending: currentLanguage === "zh" ? "准备调用工具" : "Preparing tool call",
    approved: currentLanguage === "zh" ? "工具已批准" : "Tool approved",
    denied: currentLanguage === "zh" ? "工具已拒绝" : "Tool denied",
    executing: currentLanguage === "zh" ? "正在执行" : "Running",
    failed: currentLanguage === "zh" ? "执行失败" : "Failed",
  };
  return labels[key] || (currentLanguage === "zh" ? "工具调用" : "Tool call");
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
    parts.push(currentLanguage === "zh" ? `调用 ${totalTools} 个工具` : `${totalTools} tools`);
  }
  if (commandCount > 0) {
    parts.push(currentLanguage === "zh" ? `执行 ${commandCount} 个命令` : `${commandCount} commands`);
  }
  if (changedFiles > 0) {
    parts.push(currentLanguage === "zh" ? `修改 ${changedFiles} 个文件` : `${changedFiles} files changed`);
  }
  if (subagents.length > 0) {
    parts.push(currentLanguage === "zh" ? `${subagents.length} 个子代理` : `${subagents.length} subagents`);
  }
  if (verifierChecks > 0) {
    parts.push(currentLanguage === "zh" ? `${verifierChecks} 项验证` : `${verifierChecks} checks`);
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
  const shouldSuppressInlineCode = shouldSuppressInlineAssistantCode(text, diffs);
  const visibleText = shouldSuppressInlineCode
    ? visibleAssistantWorkspaceNotice()
    : cleanedText;

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

  const processMarkup = visibleProcess.length
    ? `
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
    `
    : "";

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
                ${delegate.purpose ? `<div class="codex-delegate-line"><span class="codex-delegate-key">${escapeHtml(zhLabel("职责", "Purpose"))}</span><span class="codex-delegate-value">${escapeHtml(delegate.purpose)}</span></div>` : ""}
                ${delegate.input ? `<div class="codex-delegate-line"><span class="codex-delegate-key">${escapeHtml(zhLabel("输入", "Input"))}</span><span class="codex-delegate-value">${escapeHtml(delegate.input)}</span></div>` : ""}
                ${delegate.output ? `<div class="codex-delegate-line"><span class="codex-delegate-key">${escapeHtml(zhLabel("输出", "Output"))}</span><span class="codex-delegate-value">${escapeHtml(delegate.output)}</span></div>` : ""}
              </div>
            </details>
          `)
          .join("")}
      </div>
    `
    : "";

  const subagentMarkup = inlineResearchDelegateDetails && streamingInlineRuntime && subagents.length
    ? `
      <div class="codex-delegate-list codex-steps-list codex-subagent-list">
        ${subagents
          .map((subagent, index) => `
            <details class="codex-delegate-card codex-subagent-card"${isStreaming ? (index === subagents.length - 1 ? " open" : "") : ""}>
              <summary class="codex-delegate-summary">
                <span class="codex-delegate-name">${escapeHtml(subagent.name || "subagent")}</span>
                <span class="codex-delegate-pill">${escapeHtml(renderDelegateStatus(subagent.status || ""))}</span>
              </summary>
              <div class="codex-delegate-body">
                ${subagent.purpose ? `<div class="codex-delegate-line"><span class="codex-delegate-key">${escapeHtml(zhLabel("职责", "Purpose"))}</span><span class="codex-delegate-value">${escapeHtml(subagent.purpose)}</span></div>` : ""}
                ${subagent.input ? `<div class="codex-delegate-line"><span class="codex-delegate-key">${escapeHtml(zhLabel("输入", "Input"))}</span><span class="codex-delegate-value">${escapeHtml(subagent.input)}</span></div>` : ""}
                ${subagent.output ? `<div class="codex-delegate-line"><span class="codex-delegate-key">${escapeHtml(zhLabel("输出", "Output"))}</span><span class="codex-delegate-value">${escapeHtml(subagent.output)}</span></div>` : ""}
                ${
                  Array.isArray(subagent.evidence) && subagent.evidence.length
                    ? `<div class="codex-delegate-line"><span class="codex-delegate-key">${escapeHtml(currentLanguage === "zh" ? "证据" : "Evidence")}</span><span class="codex-delegate-value">${subagent.evidence.map((item) => escapeHtml(String(item || ""))).join("<br>")}</span></div>`
                    : ""
                }
              </div>
            </details>
          `)
          .join("")}
      </div>
    `
    : "";

  const verifierMarkup = inlineResearchDelegateDetails && streamingInlineRuntime && verifierReport
    ? `
      <details class="codex-delegate-card codex-verifier-card"${isStreaming ? " open" : ""}>
        <summary class="codex-delegate-summary">
          <span class="codex-delegate-name">${escapeHtml(currentLanguage === "zh" ? "验证器" : "Verifier")}</span>
          <span class="codex-delegate-pill">${escapeHtml(renderDelegateStatus(verifierReport.status || ""))}</span>
        </summary>
        <div class="codex-delegate-body">
          ${verifierReport.summary ? `<div class="codex-delegate-line"><span class="codex-delegate-key">${escapeHtml(currentLanguage === "zh" ? "总结" : "Summary")}</span><span class="codex-delegate-value">${escapeHtml(verifierReport.summary)}</span></div>` : ""}
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
      </details>
    `
    : "";

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
                ${subagent.purpose ? `<div class="research-runtime-line"><span>${escapeHtml(zhLabel("职责", "Purpose"))}</span><strong>${escapeHtml(subagent.purpose)}</strong></div>` : ""}
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
          ${verifierReport.summary ? `<div class="research-runtime-line"><span>${escapeHtml(zhLabel("总结", "Summary"))}</span><strong>${escapeHtml(verifierReport.summary)}</strong></div>` : ""}
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

  const runtimeTimelineMarkup = inlineResearchDelegateDetails && streamingInlineRuntime && timeline.length
    ? renderRuntimeTimeline(timeline, {
        limit: isStreaming ? 8 : 18,
        title: currentLanguage === "zh" ? "运行时间线" : "Runtime timeline",
        forceOpen: isStreaming,
      })
    : "";

  const toolMarkup = visibleTools.length
    ? `
      <div class="codex-tool-list codex-steps-list">
        ${visibleTools
          .map((tool) => {
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
            return `
              <div class="codex-tool-card codex-tool-step codex-tool-${escapeHtml(String(tool.status || "pending"))}">
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

  const diffMarkup = visibleDiffs.length
    ? `
      <div class="codex-diff-list codex-steps-list">
        <div class="codex-diff-summary">
          <span class="codex-diff-summary-title">${escapeHtml(
            currentLanguage === "zh"
              ? `已修改 ${visibleDiffs.length} 个文件`
              : `${visibleDiffs.length} files changed`,
          )}</span>
          <span class="codex-diff-summary-stats">+${escapeHtml(String(visibleDiffs.reduce((sum, diff) => sum + Number(diff.added || 0), 0)))} / -${escapeHtml(String(visibleDiffs.reduce((sum, diff) => sum + Number(diff.removed || 0), 0)))}</span>
        </div>
        ${visibleDiffs
          .map((diff) => {
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
                  <div class="codex-diff-meta">${escapeHtml(currentLanguage === "zh" ? "Agent 刚刚修改了这个文件" : "Agent just updated this file")}</div>
                  <details class="codex-diff-review-panel"${reviewOpen ? " open" : ""}>
                    <summary class="codex-diff-review-summary" data-review-path="${escapeHtml(diff.path)}">${escapeHtml(currentLanguage === "zh" ? "查看差异" : "View diff")}</summary>
                    <div class="codex-diff-review-body">${reviewMarkup || `<div class="review-detail-empty">${escapeHtml(t("reviewLoading"))}</div>`}</div>
                  </details>
                </div>
              </div>
            `;
          })
          .join("")}
      </div>
    `
    : "";

  const permissionMarkup = streamingInlineRuntime && permission
    ? `
      <div class="codex-approval-card codex-tool-step codex-approval-step">
        <div class="codex-step-rail" aria-hidden="true">
          <span class="codex-step-dot"></span>
        </div>
        <div class="codex-approval-copy">
          <div class="codex-approval-title">${escapeHtml(currentLanguage === "zh" ? "等待工具批准" : "Awaiting tool approval")}</div>
          <div class="codex-approval-meta">${escapeHtml(permission.name || "")} 路 ${escapeHtml(permission.risk || "")}</div>
        </div>
        <div class="codex-approval-actions">
          <button class="codex-approval-button" type="button" data-permission-action="deny">${escapeHtml(currentLanguage === "zh" ? "拒绝" : "Deny")}</button>
          <button class="codex-approval-button is-primary" type="button" data-permission-action="approve">${escapeHtml(currentLanguage === "zh" ? "批准" : "Approve")}</button>
        </div>
      </div>
    `
    : "";

  const worklogMarkup = isStreaming && Array.isArray(turn?.worklog) && turn.worklog.length
    ? `
      <div class="codex-worklog-list">
        ${turn.worklog
          .slice(isStreaming ? -4 : -6)
          .map((entry) => `
            <div class="codex-worklog-item codex-worklog-${escapeHtml(String(entry.kind || "activity"))}">
              <div class="codex-worklog-copy">${renderInlineMarkdown(entry.text || "")}</div>
            </div>
          `)
          .join("")}
      </div>
    `
    : "";

  const thinkingMarkup = thinking.length
    ? thinking
        .map(
          (block, index) => `
            <details class="thinking-block codex-thinking-block"${block.collapsed ? "" : " open"}>
              <summary>${escapeHtml(currentLanguage === "zh" ? `Thinking ${index + 1}` : `Thinking ${index + 1}`)}</summary>
              <div class="thinking-content markdown-body">${renderMarkdown(block.content || "")}</div>
            </details>
          `,
        )
        .join("")
    : "";

  const streamingParts = isStreaming
    ? renderStreamingAssistantContent(visibleText, {
        placeholder: pendingAssistantPlaceholderText(),
      })
    : null;

  const textMarkup = isStreaming
    ? `
      <div class="codex-answer codex-answer-streaming${streamingParts?.isEmpty ? " codex-answer-empty" : ""}">
        <div class="codex-streaming-text markdown-body" data-streaming-markdown>
          <div class="codex-streaming-stable" data-streaming-stable${streamingParts?.stableText ? "" : " hidden"}>${streamingParts?.stableHtml || ""}</div>
          <span class="codex-streaming-tail" data-streaming-tail${streamingParts?.tailVisible ? "" : " hidden"}>${streamingParts?.tailHtml || ""}</span>
        </div>
      </div>
    `
    : visibleText
      ? `<div class="codex-answer markdown-body">${renderMarkdown(visibleText)}</div>`
      : "";

  return `
    <article class="message-row assistant-row assistant-message-row codex-turn-row">
      <div class="codex-turn-shell">
        ${runtimeHead}
        ${runtimeSummaryMarkup}
        ${processMarkup}
        ${delegateMarkup}
        ${subagentMarkup}
        ${verifierMarkup}
        ${checkpointMarkup}
        ${runtimeTimelineMarkup}
        ${worklogMarkup}
        ${thinkingMarkup}
        ${textMarkup}
        ${permissionMarkup}
        ${diffMarkup}
      </div>
    </article>
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

function appendAssistantBubble(content) {
  if (!messageStream) return null;
  messageStream.classList.remove("is-empty");
  messageStream.querySelector(".empty-state")?.remove();
  const row = document.createElement("article");
  row.className = "codex-turn-anchor";
  if (!activeAssistantTurn) resetActiveAssistantTurn();
  activeAssistantTurn.text = content || "";
  messageStream.appendChild(row);
  pendingAssistantBubble = row;
  refreshPendingAssistantBubble();
  scrollMessageStreamToBottom(true);
  return row;
}

function updateAssistantBubble(content) {
  if (!pendingAssistantBubble) {
    pendingAssistantBubble = appendAssistantBubble(content);
  }
  if (!activeAssistantTurn) resetActiveAssistantTurn();
  const deltaContent = sanitizeMessageContent(String(content || ""));
  const currentContent = String(activeAssistantTurn.text || "");
  activeAssistantTurn.receivedDelta = true;
  if (!currentContent) {
    activeAssistantTurn.text = deltaContent;
  } else if (!deltaContent) {
    activeAssistantTurn.text = currentContent;
  } else if (deltaContent === currentContent) {
    activeAssistantTurn.text = currentContent;
  } else if (deltaContent.startsWith(currentContent)) {
    activeAssistantTurn.text = deltaContent;
  } else if (currentContent.startsWith(deltaContent)) {
    activeAssistantTurn.text = currentContent;
  } else {
    const overlapLimit = Math.min(currentContent.length, deltaContent.length);
    let overlap = 0;
    for (let index = overlapLimit; index > 0; index -= 1) {
      if (currentContent.endsWith(deltaContent.slice(0, index))) {
        overlap = index;
        break;
      }
    }
    const commonPrefix = (() => {
      const limit = Math.min(currentContent.length, deltaContent.length);
      let count = 0;
      while (count < limit && currentContent[count] === deltaContent[count]) {
        count += 1;
      }
      return count;
    })();
    const looksLikeSnapshot =
      commonPrefix >= 24 &&
      commonPrefix * 10 >= Math.min(currentContent.length, deltaContent.length) * 7;
    activeAssistantTurn.text = looksLikeSnapshot
      ? deltaContent
      : `${currentContent}${deltaContent.slice(overlap)}`;
  }
  schedulePendingAssistantTextSync();
  if (messageStream) {
    scrollMessageStreamToBottom(true);
  }
}

function createThinkingBlock(message) {
  const row = document.createElement("article");
  row.className = "message-row assistant-row assistant-message-row";
  const details = document.createElement("details");
  details.className = "thinking-block";
  details.open = !message.collapsed;
  details.innerHTML = `
    <summary>Thinking</summary>
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
    ? {
        ...mergeAssistantTurnData(finalAssistantTurn.data, runtimeTurn, { preferLiveText: false }),
        text: String(finalAssistantTurn.data?.text || runtimeTurn?.text || ""),
        receivedDelta: false,
      }
    : finalAssistantTurn.data;
  const keepBottom = isNearMessageStreamBottom();
  pendingAssistantBubble.innerHTML = renderAssistantTurn(finalData, { streaming: false });
  bindTurnInteractionHandlers(pendingAssistantBubble);
  resetPendingAssistantRenderState();
  pendingAssistantBubble = null;
  pendingUserBubble = null;
  if (keepBottom) {
    scrollMessageStreamToBottom(true);
  }
  return true;
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
      currentAssistant.text = combineAssistantSegments(
        currentAssistant.text || "",
        displayMarkdownText(message.content || ""),
      );
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
      const callId = message.call_id || `${Date.now()}`;
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
      return;
    }

    if (message.kind === "tool_result") {
      const tool = currentAssistant.tools.find((item) => item.call_id === message.call_id);
      if (tool) {
        tool.result = message.content || "";
        tool.success = message.success ?? null;
        tool.status = message.status || tool.status || "complete";
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
  if (!Array.isArray(turns) || !activeAssistantTurn) return turns;
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
        showToast(currentLanguage === "zh" ? "已撤销本次修改" : "Edit undone");
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
}

function renderEmptyState() {
  const researchText = currentLanguage === "zh" ? "今天想研究点什么？" : "What would you like to explore today?";
  const chatText = currentLanguage === "zh" ? "告诉我你的想法" : "Tell me what you're thinking";
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
      messageStream.appendChild(wrapper);
    }
  });
  bindTurnInteractionHandlers(messageStream);
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
  if (!preservePanelVisibility) {
    ensureCodePanelVisible();
    document.querySelector(".workspace-code-panel")?.classList.remove("is-closed");
  }
  workspaceCodePath.textContent = file.path || file.name || "";
  workspaceCodeMeta.textContent = workspaceFileMetaText(file);
  updateWorkspaceCodeView();
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
              : `<div class="review-detail-empty">${escapeHtml(currentLanguage === "zh" ? "该产物为二进制文件，请在工作区中打开预览。" : "This artifact is binary. Open it in the workspace preview.")}</div>`
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
    deep_learning: currentLanguage === "zh" ? "深度学习研究" : "Deep learning",
    experimental_design: currentLanguage === "zh" ? "实验设计" : "Experimental design",
    literature_review: currentLanguage === "zh" ? "文献综述" : "Literature review",
    simulation: currentLanguage === "zh" ? "仿真研究" : "Simulation",
    data_analysis: currentLanguage === "zh" ? "数据分析" : "Data analysis",
    adaptive_research: currentLanguage === "zh" ? "自适应研究" : "Adaptive research",
  };
  return labels[key] || (currentLanguage === "zh" ? "研究流程" : "Research workflow");
}

function researchStateLabel(state) {
  const key = String(state || "").trim().toLowerCase();
  const labels = {
    active: currentLanguage === "zh" ? "进行中" : "Active",
    blocked: currentLanguage === "zh" ? "受阻" : "Blocked",
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
              ${cleanDisplayText(item.purpose) ? `<div class="research-runtime-line"><span>${escapeHtml(zhLabel("职责", "Purpose"))}</span><strong>${escapeHtml(cleanDisplayText(item.purpose))}</strong></div>` : ""}
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
        ${cleanDisplayText(report.summary) ? `<div class="research-runtime-line"><span>${escapeHtml(currentLanguage === "zh" ? "总结" : "Summary")}</span><strong>${escapeHtml(cleanDisplayText(report.summary))}</strong></div>` : ""}
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

function renderResearch(research) {
  if (!researchPanel) return;
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
      ${resourceSummary ? `<div class="research-meta">${escapeHtml(zhLabel("当前资源", "Resources"))}: ${escapeHtml(resourceSummary)}</div>` : ""}
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
  const blocker = cleanDisplayText(research?.blocker || "");
  researchFloatingBody.innerHTML = `
    <div class="research-floating-topic">${escapeHtml(topic)}</div>
    <div class="research-floating-meta">${escapeHtml(researchWorkflowLabel(research?.workflow_kind))} / ${escapeHtml(researchStateLabel(research?.overall_state))} / ${escapeHtml(cleanDisplayText(research?.phase || ""))}</div>
    ${blocker ? `<div class="research-review-item">${escapeHtml(blocker)}</div>` : ""}
    ${graphMarkup.html}
    ${renderResearchRuntimeSubagents(runtimeSubagents, { limit: 2, outputLimit: 120, evidenceLimit: 2 })}
    ${renderResearchRuntimeVerifier(runtimeVerifier, { checkLimit: 2, issueLimit: 2 })}
    ${renderRuntimeTimeline(runtimeTimeline, { limit: 4, title: currentLanguage === "zh" ? "时间线" : "Timeline" })}
    <div class="research-review-list">
      ${resumePoints.map((item) => `<div class="research-review-item">${escapeHtml(item)}</div>`).join("")}
      ${reviewItems.map((item) => `<div class="research-review-item">${escapeHtml(item)}</div>`).join("")}
    </div>
  `;
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
    api_url: String(primaryApiUrl?.value || config.api_url || "").trim(),
    deep_think: false,
    reasoning_effort: currentEffort,
    competition_mode: Boolean(competitionMode?.checked),
    privacy_mode: Boolean(privacyMode?.checked),
    workspace_root: String(runtimeWorkspaceRoot?.value || config.workspace_root || "").trim(),
    api_key: String(runtimeApiKey?.value || "").trim() || null,
    auto_approve_tools: Boolean(autoApproveTools?.checked),
    max_auto_approve_risk: String(getSegmentedValue(riskBoundary, config.max_auto_approve_risk || "safe")).trim().toLowerCase(),
    max_tool_calls_per_minute: parseLimitValue(maxToolCalls, "30"),
    burst_limit: parseLimitValue(burstLimit, "5"),
    toolchains,
  };
}

function syncSettingsFromConfig(config) {
  if (!config) return;
  if (primaryApiUrl) primaryApiUrl.value = config.api_url || "";
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
      "30",
    )
  );
  setSegmentedValue(
    burstLimit,
    normalizeChoice(
      config.burst_limit === 0 ? "unlimited" : String(config.burst_limit ?? 5),
      ["1", "5", "unlimited"],
      "5",
    )
  );
  if (runtimeWorkspaceRoot) runtimeWorkspaceRoot.value = config.workspace_root || "";
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
  if (workspaceTitle) workspaceTitle.textContent = "";
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
      parts.push(`${status.upstream} 路 ${template("gitAheadBehind", { ahead: Number(status.ahead) || 0, behind: Number(status.behind) || 0 })}`);
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
  return parts.join(" 路 ");
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
            ? `<div class="git-empty">${escapeHtml(currentLanguage === "zh" ? "正在加载差异..." : "Loading diff...")}</div>`
            : workingDiff
              ? renderHighlightedCodeBlock(workingDiff, "diff")
              : `<div class="git-empty">${escapeHtml(t("gitNoDiff"))}</div>`
        }</div>
      </article>
      <article class="git-panel">
        <div class="git-panel-head">${escapeHtml(t("gitDiffStaged"))}</div>
        <div class="git-diff-block markdown-body">${
          !diffLoaded
            ? `<div class="git-empty">${escapeHtml(currentLanguage === "zh" ? "正在加载差异..." : "Loading diff...")}</div>`
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
    gitGraphView.innerHTML = `<div class="git-empty">${escapeHtml(currentLanguage === "zh" ? "正在加载 Graph..." : "Loading graph...")}</div>`;
    return;
  }
    gitGraphView.innerHTML = `<div class="git-empty">${escapeHtml(currentLanguage === "zh" ? "正在加载 Graph..." : "Loading graph...")}</div>`;
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
  currentMainView = nextView === "git" ? "git" : "chat";
  const workspaceChat = document.querySelector(".workspace-chat");
  const conversationStage = document.querySelector(".conversation-stage");
  const composer = document.querySelector(".composer-shell");

  if (workspaceChat) workspaceChat.hidden = currentMainView === "git";
  if (gitWorkspace) gitWorkspace.hidden = currentMainView !== "git";
  if (conversationStage) conversationStage.hidden = false;
  if (composer) composer.hidden = currentMainView === "git";
  if (gitNav) {
    gitNav.classList.toggle("is-active", currentMainView === "git");
  }
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
  renderTerminalDrawer();
}

function applyBootstrap(data) {
  const previousWorkspaceRoot = currentWorkspaceRoot;
  const showSandboxNotice = shouldShowSandboxNotice(data?.sandbox);
  bootstrapData = data;
  syncAcceptedDiffStatusesFromMessages(data?.messages || []);
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
  const review = data?.review || buildReviewFromMessages(data?.messages || []);
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
    langToggle.textContent = currentLanguage === "zh" ? "English" : "中文";
  updateEffortUI();
  syncWorkspaceCodeRenderToggle();
  applyWorkspaceMode(currentWorkspaceMode);
  renderExtensionList(extensionSearchInput?.value || "");
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
    currentMainView = currentMainView === "git" ? "git" : currentMainView;
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
    const shouldPreserveVisible =
      currentVisibleSessionId &&
      currentVisibleSessionId === nextVisibleSessionId &&
      isVisibleSessionRunning();

    if (shouldPreserveVisible && bootstrapData) {
      const preservedMessages = bootstrapData.messages || [];
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
      currentMainView = currentMainView === "git" ? "git" : currentMainView;
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

  if (event.type === "assistant_delta") {
    if (!isVisibleSession) return;
    if (!activeAssistantTurn) {
      resetActiveAssistantTurn();
    }
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
    refreshPendingAssistantBubble();
    schedulePendingAssistantStatusSync();
    return;
  }

  if (event.type === "messages" || event.type === "complete") {
    const visibleMessages = visibleConversationMessages(event.messages || []);
    if (sessionId) {
      const targetSession = (bootstrapData?.sessions || []).find((session) => session.id === sessionId);
      if (targetSession) {
        targetSession.updated_at = new Date().toISOString();
        targetSession.message_count = visibleMessages.length;
        const nextSummary = latestConversationSummary(visibleMessages, 42);
        targetSession.summary = nextSummary || targetSession.summary || "";
      }
    }

    if (event.type === "complete" && sessionId) {
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
    const label = event.activity?.label || "";
    const detail = event.activity?.detail || "";
    const phase = event.activity?.phase || "";
    const status = event.activity?.status || "";
    const meta = event.activity?.meta || "";
    const agent = event.activity?.agent || "";
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
    upsertToolEntry(tool);
    pushAssistantWorklog(describeToolWorklog(tool));
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
    files.forEach((file) => pushAssistantWorklog(describeEditedFileWorklog(file)));
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
    pushAssistantWorklog({
      kind: "approval",
      text: currentLanguage === "zh"
        ? "这里触发了需要人工批准的操作，我先停下来等你确认。"
        : "This step triggered a manual approval request, so I’m pausing here for your confirmation.",
      dedupeKey: "permission:required",
    });
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
  const targetSessionId = await ensureSessionReady();
  if (!targetSessionId) {
    showToast(classifyAppError(new Error("session not ready"), "send").message);
    return;
  }

  isSending = true;
  suppressVisibleStreamBootstrap = true;
  if (messageInput) messageInput.disabled = true;
  setStopButtonVisible(true);
  startActivity(currentLanguage === "zh" ? "正在思考" : "Thinking");
  liveEditedFiles = [];
  liveProcessEvents = [];
  pinnedEditedFiles = [];
  pendingPermissionRequest = null;
  renderAgentRuntimeStrip();
  renderAgentProcessStrip();
  renderPermissionStrip();

  try {
    const parsedInput = parseAgentInputProtocol(content);
    if (!parsedInput.outbound) {
      if (currentWorkspaceMode === "research") {
        showToast(currentLanguage === "zh" ? "请输入 /spec 后的研究课题内容" : "Add a research topic after /spec.");
      }
      throw new Error("empty outbound content");
    }
    const userText = sanitizeMessageContent(parsedInput.display);
    const fileLines = pendingFiles.length
      ? `\n\n[ATTACHED_FILES]\n${pendingFiles
          .map((file) => file.path || file.name)
          .filter(Boolean)
          .join("\n")}`
      : "";
    const outbound = `${parsedInput.outbound}${fileLines}`.trim();
    const mode = parsedInput.mode;
    pendingResearchStart = parsedInput.forceResearch || mode === "research" || currentWorkspaceMode === "research";

    if (pendingUserBubble) pendingUserBubble.remove();
    pendingUserBubble = appendUserBubble(userText);

    if (pendingAssistantBubble) pendingAssistantBubble.remove();
    resetPendingAssistantRenderState();
    clearPendingAssistantFrames();
    resetActiveAssistantTurn();
    activeAssistantTurn.startedAt = Date.now();
    activeAssistantTurn.activity = mode === "research"
      ? (currentLanguage === "zh" ? "正在研究" : "Researching")
      : t("activityReviewing");
    pendingAssistantBubble = appendAssistantBubble("");
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
    const response = await hostClient.chat.stream({ content: outbound, mode, language: currentLanguage });

    if (messageInput) {
      messageInput.value = "";
    }
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
    !event.target.closest(".session-floating-menu") &&
    !event.target.closest(".dock-panel-grip") &&
    !event.target.closest(".panel-floating-menu")
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

window.addEventListener("resize", () => {
  stopDockDrag();
  stopResizerDrag();
  if (activeSessionMenuAnchor && !sessionMenu?.hidden) {
    positionSessionMenu(activeSessionMenuAnchor);
  }
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
  grip.addEventListener("click", (event) => {
    if (suppressNextGripClick) {
      suppressNextGripClick = false;
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    openPanelMenu(panelId, grip);
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
  pendingFiles = Array.from(fileInput.files || []).map((file) => ({
    name: file.name,
    path: file.name,
  }));
  renderPendingFiles();
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
      setActivityPanel(null, { preserveMainView: currentMainView === "git" });
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
    setActivityPanel(null, { preserveMainView: currentMainView === "git" });
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
  if (!currentStreamingSessionId) return;
  try {
    await hostClient.chat.stop(currentStreamingSessionId);
  } catch (error) {
    console.error(error);
  } finally {
    endSessionRun(currentStreamingSessionId);
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
