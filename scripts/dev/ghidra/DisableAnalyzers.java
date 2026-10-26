// Pré-script headless : désactive les analyseurs nommés avant l'auto-analyse, et échoue si un nom n'existe
// pas ou reste actif (même contrôle que REA, `ReaGhidraAnalysisOptions.java`). Sur BDS, couper les analyseurs
// lents ramène l'analyse de plus de 3 h (inachevée) à environ 2 h 30 ; voir README.md.
//
// Argument : liste de noms exacts séparés par des virgules.
// @category mc-rs

import java.util.Map;

import ghidra.app.plugin.core.analysis.AutoAnalysisManager;
import ghidra.app.util.headless.HeadlessScript;

public class DisableAnalyzers extends HeadlessScript {

    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        if (args.length != 1 || args[0].isBlank()) {
            throw new IllegalArgumentException("usage : DisableAnalyzers \"Nom 1,Nom 2,...\"");
        }
        AutoAnalysisManager analysis = AutoAnalysisManager.getAnalysisManager(currentProgram);
        for (String raw : args[0].split(",")) {
            String name = raw.trim();
            if (analysis.getAnalyzer(name) == null) {
                throw new IllegalArgumentException("analyseur inconnu pour ce programme : " + name);
            }
            setAnalysisOption(currentProgram, name, "false");
        }
        Map<String, String> options = getCurrentAnalysisOptionsAndValues(currentProgram);
        for (String raw : args[0].split(",")) {
            String name = raw.trim();
            if (!"false".equals(options.get(name))) {
                throw new IllegalStateException("analyseur resté actif : " + name);
            }
            println("désactivé : " + name);
        }
    }
}
