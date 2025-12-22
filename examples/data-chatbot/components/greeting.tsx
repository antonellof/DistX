import { motion } from "framer-motion";
import { DatabaseIcon, FileSpreadsheetIcon, FileTextIcon, FileIcon, BookOpenIcon } from "lucide-react";

export const Greeting = () => {
  return (
    <div
      className="mx-auto mt-4 flex size-full max-w-3xl flex-col justify-center px-4 md:mt-16 md:px-8"
      key="overview"
    >
      <motion.div
        animate={{ opacity: 1, y: 0 }}
        className="font-semibold text-xl md:text-2xl mb-4"
        exit={{ opacity: 0, y: 10 }}
        initial={{ opacity: 0, y: 10 }}
        transition={{ delay: 0.5 }}
      >
        vectX Data Chatbot
      </motion.div>
      <motion.div
        animate={{ opacity: 1, y: 0 }}
        className="text-lg text-zinc-500 md:text-xl mb-6"
        exit={{ opacity: 0, y: 10 }}
        initial={{ opacity: 0, y: 10 }}
        transition={{ delay: 0.6 }}
      >
        Query your structured data with natural language using vectX similarity engine.
      </motion.div>
      
      <motion.div
        animate={{ opacity: 1, y: 0 }}
        className="space-y-4"
        exit={{ opacity: 0, y: 10 }}
        initial={{ opacity: 0, y: 10 }}
        transition={{ delay: 0.7 }}
      >
        <div className="text-sm font-medium text-foreground mb-2">
          Supported File Formats:
        </div>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3 mb-4">
          <div className="space-y-2">
            <div className="text-xs font-medium text-foreground">Structured Data:</div>
            <div className="grid grid-cols-2 gap-2">
              <div className="flex items-center gap-2 p-2 rounded-lg border bg-muted/50">
                <FileSpreadsheetIcon className="h-4 w-4 text-blue-500" />
                <div>
                  <div className="font-medium text-xs">CSV</div>
                </div>
              </div>
              <div className="flex items-center gap-2 p-2 rounded-lg border bg-muted/50">
                <FileTextIcon className="h-4 w-4 text-green-500" />
                <div>
                  <div className="font-medium text-xs">Excel</div>
                </div>
              </div>
            </div>
          </div>
          <div className="space-y-2">
            <div className="text-xs font-medium text-foreground">Documents:</div>
            <div className="grid grid-cols-2 gap-2">
              <div className="flex items-center gap-2 p-2 rounded-lg border bg-muted/50">
                <FileIcon className="h-4 w-4 text-red-500" />
                <div>
                  <div className="font-medium text-xs">PDF</div>
                </div>
              </div>
              <div className="flex items-center gap-2 p-2 rounded-lg border bg-muted/50">
                <BookOpenIcon className="h-4 w-4 text-purple-500" />
                <div>
                  <div className="font-medium text-xs">Word/TXT</div>
                </div>
              </div>
            </div>
          </div>
        </div>
        
        <div className="text-sm text-muted-foreground mt-4 space-y-2">
          <p>
            <strong>How it works:</strong>
          </p>
          <ul className="list-disc list-inside space-y-1 ml-2">
            <li>Upload CSV/Excel data or PDF/Word documents via 📎 or sidebar</li>
            <li>vectX auto-detects schemas and embeds content for semantic search</li>
            <li>Query naturally: "Find products like iPhone" or "What does the doc say about pricing?"</li>
            <li>Get explainable results with similarity scores</li>
          </ul>
        </div>
      </motion.div>
    </div>
  );
};
