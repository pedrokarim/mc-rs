// Décompile toutes les fonctions qui référencent une chaîne donnée.
//
// Usage (projet Ghidra déjà analysé, voir scripts/dev/ghidra/README.md) :
//   analyzeHeadless <dossier_projets> <projet> -process <programme> -noanalysis -readOnly
//     -scriptPath scripts/dev/ghidra -postScript DecompileStringRefs.java <dossier_sortie> <chaîne>...
//
// Pour chaque chaîne : recherche de ses occurrences (ASCII, terminées par un octet nul) dans la mémoire du
// programme, références vers chacune, puis décompilation de chaque fonction appelante. Écrit un fichier
// `<adresse>.c` par fonction et un `index.txt` (chaîne → occurrence → fonctions).
//@category mc-rs

import java.io.File;
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.util.LinkedHashMap;
import java.util.Map;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.mem.Memory;
import ghidra.program.model.symbol.Reference;

public class DecompileStringRefs extends GhidraScript {

    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        if (args.length < 2) {
            printerr("arguments : <dossier_sortie> <chaîne>...");
            return;
        }
        File out = new File(args[0]);
        out.mkdirs();

        DecompInterface decompiler = new DecompInterface();
        decompiler.openProgram(currentProgram);
        Memory memory = currentProgram.getMemory();
        Map<Address, Function> functions = new LinkedHashMap<>();

        try (PrintWriter index = new PrintWriter(new File(out, "index.txt"), StandardCharsets.UTF_8)) {
            for (int i = 1; i < args.length; i++) {
                String needle = args[i];
                byte[] bytes = (needle + "\0").getBytes(StandardCharsets.US_ASCII);
                index.println("== " + needle);
                Address from = memory.getMinAddress();
                while (from != null && !monitor.isCancelled()) {
                    Address hit = memory.findBytes(from, bytes, null, true, monitor);
                    if (hit == null) {
                        break;
                    }
                    Reference[] refs = getReferencesTo(hit);
                    index.println("  occurrence " + hit + " : " + refs.length + " référence(s)");
                    for (Reference ref : refs) {
                        Function f = getFunctionContaining(ref.getFromAddress());
                        String owner = f == null ? "(hors fonction)" : f.getEntryPoint() + " " + f.getName(true);
                        index.println("    depuis " + ref.getFromAddress() + " dans " + owner);
                        if (f != null) {
                            functions.putIfAbsent(f.getEntryPoint(), f);
                        }
                    }
                    from = hit.next();
                }
            }
        }

        for (Function f : functions.values()) {
            if (monitor.isCancelled()) {
                break;
            }
            DecompileResults res = decompiler.decompileFunction(f, 300, monitor);
            File file = new File(out, f.getEntryPoint().toString() + ".c");
            try (PrintWriter w = new PrintWriter(file, StandardCharsets.UTF_8)) {
                w.println("// " + f.getName(true) + " @ " + f.getEntryPoint());
                if (res.decompileCompleted()) {
                    w.println(res.getDecompiledFunction().getC());
                } else {
                    w.println("// décompilation échouée : " + res.getErrorMessage());
                }
            }
        }
        decompiler.dispose();
        println(functions.size() + " fonction(s) décompilée(s) dans " + out);
    }
}
