// Décompile les méthodes virtuelles d'une classe C++ à partir de sa vtable RTTI.
//
// Usage (projet Ghidra déjà analysé, voir scripts/dev/ghidra/README.md) :
//   analyzeHeadless <dossier_projets> <projet> -process <programme> -noanalysis -readOnly
//     -scriptPath scripts/dev/ghidra -postScript DecompileVtable.java <dossier_sortie> <classe>...
//
// L'analyseur RTTI de Ghidra nomme les vtables `<Classe>::vftable` (Windows) ; pour chaque symbole trouvé,
// les entrées sont lues comme pointeurs 64 bits jusqu'au premier qui ne désigne pas une fonction. Écrit
// `<classe>/<rang>_<adresse>.c` et `<classe>/vtable.txt` (rang → fonction).
//@category mc-rs

import java.io.File;
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.Symbol;
import ghidra.program.model.symbol.SymbolIterator;

public class DecompileVtable extends GhidraScript {

    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        if (args.length < 2) {
            printerr("arguments : <dossier_sortie> <classe>...");
            return;
        }
        File out = new File(args[0]);
        DecompInterface decompiler = new DecompInterface();
        decompiler.openProgram(currentProgram);

        for (int i = 1; i < args.length; i++) {
            String cls = args[i];
            File dir = new File(out, cls.replace("::", "__"));
            dir.mkdirs();
            int vtables = 0;
            try (PrintWriter index = new PrintWriter(new File(dir, "vtable.txt"), StandardCharsets.UTF_8)) {
                SymbolIterator it = currentProgram.getSymbolTable().getSymbols("vftable");
                while (it.hasNext() && !monitor.isCancelled()) {
                    Symbol s = it.next();
                    if (!s.getParentNamespace().getName(true).equals(cls)) {
                        continue;
                    }
                    vtables++;
                    Address table = s.getAddress();
                    index.println("== vtable " + table + " (" + s.getName(true) + ")");
                    for (int slot = 0; slot < 512; slot++) {
                        Address entry = table.add(8L * slot);
                        Address target = toAddr(getLong(entry));
                        Function f = getFunctionAt(target);
                        if (f == null) {
                            break;
                        }
                        index.println("  [" + slot + "] " + target + " " + f.getName(true));
                        DecompileResults res = decompiler.decompileFunction(f, 300, monitor);
                        File file = new File(dir, String.format("%03d_%s.c", slot, target));
                        try (PrintWriter w = new PrintWriter(file, StandardCharsets.UTF_8)) {
                            w.println("// " + cls + " vtable[" + slot + "] = " + f.getName(true) + " @ " + target);
                            w.println(res.decompileCompleted()
                                    ? res.getDecompiledFunction().getC()
                                    : "// décompilation échouée : " + res.getErrorMessage());
                        }
                    }
                }
            }
            println(cls + " : " + vtables + " vtable(s)");
        }
        decompiler.dispose();
    }
}
