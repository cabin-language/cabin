**Cabin Language Server started**

# Awaiting next request from client...

**Request received:**
```json
Content-Length: 3911

{"method":"initialize","params":{"rootPath":null,"clientInfo":{"name":"Neovim","version":"0.11.0-dev+g9b7905df16"},"processId":144175,"workDoneToken":"1","workspaceFolders":null,"capabilities":{"general":{"positionEncodings":["utf-8","utf-16","utf-32"]},"workspace":{"workspaceEdit":{"resourceOperations":["rename","create","delete"]},"didChangeConfiguration":{"dynamicRegistration":false},"applyEdit":true,"workspaceFolders":true,"semanticTokens":{"refreshSupport":true},"configuration":true,"inlayHint":{"refreshSupport":true},"symbol":{"dynamicRegistration":false,"symbolKind":{"valueSet":[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26]}},"didChangeWatchedFiles":{"dynamicRegistration":false,"relativePatternSupport":true}},"window":{"workDoneProgress":true,"showMessage":{"messageActionItem":{"additionalPropertiesSupport":true}},"showDocument":{"support":true}},"textDocument":{"callHierarchy":{"dynamicRegistration":false},"publishDiagnostics":{"tagSupport":{"valueSet":[1,2]},"dataSupport":true,"relatedInformation":true},"rangeFormatting":{"dynamicRegistration":true,"rangesSupport":true},"foldingRange":{"dynamicRegistration":false,"foldingRange":{"collapsedText":true},"lineFoldingOnly":true},"rename":{"dynamicRegistration":true,"prepareSupport":true},"documentSymbol":{"dynamicRegistration":false,"hierarchicalDocumentSymbolSupport":true,"symbolKind":{"valueSet":[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26]}},"diagnostic":{"dynamicRegistration":false},"codeAction":{"dynamicRegistration":true,"resolveSupport":{"properties":["edit"]},"codeActionLiteralSupport":{"codeActionKind":{"valueSet":["","quickfix","refactor","refactor.extract","refactor.inline","refactor.rewrite","source","source.organizeImports"]}},"dataSupport":true,"isPreferredSupport":true},"hover":{"dynamicRegistration":true,"contentFormat":["markdown","plaintext"]},"signatureHelp":{"dynamicRegistration":false,"signatureInformation":{"activeParameterSupport":true,"parameterInformation":{"labelOffsetSupport":true},"documentationFormat":["markdown","plaintext"]}},"typeDefinition":{"linkSupport":true},"synchronization":{"dynamicRegistration":false,"didSave":true,"willSave":true,"willSaveWaitUntil":true},"declaration":{"linkSupport":true},"definition":{"dynamicRegistration":true,"linkSupport":true},"documentHighlight":{"dynamicRegistration":false},"references":{"dynamicRegistration":false},"semanticTokens":{"dynamicRegistration":false,"requests":{"full":{"delta":true},"range":false},"formats":["relative"],"tokenModifiers":["declaration","definition","readonly","static","deprecated","abstract","async","modification","documentation","defaultLibrary"],"tokenTypes":["namespace","type","class","enum","interface","struct","typeParameter","parameter","variable","property","enumMember","event","function","method","macro","keyword","modifier","comment","string","number","regexp","operator","decorator"],"overlappingTokenSupport":true,"augmentsSyntaxTokens":true,"serverCancelSupport":false,"multilineTokenSupport":false},"implementation":{"linkSupport":true},"inlayHint":{"dynamicRegistration":true,"resolveSupport":{"properties":["textEdits","tooltip","location","command"]}},"completion":{"dynamicRegistration":false,"completionList":{"itemDefaults":["editRange","insertTextFormat","insertTextMode","data"]},"contextSupport":false,"completionItemKind":{"valueSet":[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25]},"completionItem":{"commitCharactersSupport":false,"snippetSupport":true,"resolveSupport":{"properties":["additionalTextEdits"]},"deprecatedSupport":true,"tagSupport":{"valueSet":[1]},"documentationFormat":["markdown","plaintext"],"preselectSupport":false}},"formatting":{"dynamicRegistration":true},"codeLens":{"dynamicRegistration":false,"resolveSupport":{"properties":["command"]}}}},"trace":"off","rootUri":null},"jsonrpc":"2.0","id":1}
```

**Handling request:**
```json
{"method":"initialize","params":{"rootPath":null,"clientInfo":{"name":"Neovim","version":"0.11.0-dev+g9b7905df16"},"processId":144175,"workDoneToken":"1","workspaceFolders":null,"capabilities":{"general":{"positionEncodings":["utf-8","utf-16","utf-32"]},"workspace":{"workspaceEdit":{"resourceOperations":["rename","create","delete"]},"didChangeConfiguration":{"dynamicRegistration":false},"applyEdit":true,"workspaceFolders":true,"semanticTokens":{"refreshSupport":true},"configuration":true,"inlayHint":{"refreshSupport":true},"symbol":{"dynamicRegistration":false,"symbolKind":{"valueSet":[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26]}},"didChangeWatchedFiles":{"dynamicRegistration":false,"relativePatternSupport":true}},"window":{"workDoneProgress":true,"showMessage":{"messageActionItem":{"additionalPropertiesSupport":true}},"showDocument":{"support":true}},"textDocument":{"callHierarchy":{"dynamicRegistration":false},"publishDiagnostics":{"tagSupport":{"valueSet":[1,2]},"dataSupport":true,"relatedInformation":true},"rangeFormatting":{"dynamicRegistration":true,"rangesSupport":true},"foldingRange":{"dynamicRegistration":false,"foldingRange":{"collapsedText":true},"lineFoldingOnly":true},"rename":{"dynamicRegistration":true,"prepareSupport":true},"documentSymbol":{"dynamicRegistration":false,"hierarchicalDocumentSymbolSupport":true,"symbolKind":{"valueSet":[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26]}},"diagnostic":{"dynamicRegistration":false},"codeAction":{"dynamicRegistration":true,"resolveSupport":{"properties":["edit"]},"codeActionLiteralSupport":{"codeActionKind":{"valueSet":["","quickfix","refactor","refactor.extract","refactor.inline","refactor.rewrite","source","source.organizeImports"]}},"dataSupport":true,"isPreferredSupport":true},"hover":{"dynamicRegistration":true,"contentFormat":["markdown","plaintext"]},"signatureHelp":{"dynamicRegistration":false,"signatureInformation":{"activeParameterSupport":true,"parameterInformation":{"labelOffsetSupport":true},"documentationFormat":["markdown","plaintext"]}},"typeDefinition":{"linkSupport":true},"synchronization":{"dynamicRegistration":false,"didSave":true,"willSave":true,"willSaveWaitUntil":true},"declaration":{"linkSupport":true},"definition":{"dynamicRegistration":true,"linkSupport":true},"documentHighlight":{"dynamicRegistration":false},"references":{"dynamicRegistration":false},"semanticTokens":{"dynamicRegistration":false,"requests":{"full":{"delta":true},"range":false},"formats":["relative"],"tokenModifiers":["declaration","definition","readonly","static","deprecated","abstract","async","modification","documentation","defaultLibrary"],"tokenTypes":["namespace","type","class","enum","interface","struct","typeParameter","parameter","variable","property","enumMember","event","function","method","macro","keyword","modifier","comment","string","number","regexp","operator","decorator"],"overlappingTokenSupport":true,"augmentsSyntaxTokens":true,"serverCancelSupport":false,"multilineTokenSupport":false},"implementation":{"linkSupport":true},"inlayHint":{"dynamicRegistration":true,"resolveSupport":{"properties":["textEdits","tooltip","location","command"]}},"completion":{"dynamicRegistration":false,"completionList":{"itemDefaults":["editRange","insertTextFormat","insertTextMode","data"]},"contextSupport":false,"completionItemKind":{"valueSet":[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25]},"completionItem":{"commitCharactersSupport":false,"snippetSupport":true,"resolveSupport":{"properties":["additionalTextEdits"]},"deprecatedSupport":true,"tagSupport":{"valueSet":[1]},"documentationFormat":["markdown","plaintext"],"preselectSupport":false}},"formatting":{"dynamicRegistration":true},"codeLens":{"dynamicRegistration":false,"resolveSupport":{"properties":["command"]}}}},"trace":"off","rootUri":null},"jsonrpc":"2.0","id":1}
```

*Connected to client `Neovim 0.11.0-dev+g9b7905df16`*

**Response:**
```json
Content-Length: 158

{"id":1,"jsonrpc":"2.0","result":{"capabilities":{"textDocumentSync":1,"hoverProvider":true},"serverInfo":{"name":"cabin-language-server","version":"0.0.1"}}}
```

# Awaiting next request from client...

**Request received:**
```json
Content-Length: 52

{"method":"initialized","params":{},"jsonrpc":"2.0"}
```

**Handling request:**
```json
{"method":"initialized","params":{},"jsonrpc":"2.0"}
```

**No response needed.**

# Awaiting next request from client...

**Request received:**
```json
Content-Length: 1204

{"method":"textDocument/didOpen","params":{"textDocument":{"text":"# The Cabin prelude. This code is placed implicitly at the beginning of all Cabin files, aside from the standard library.\n# It just brings some commonly used names into scope.\n\n# Anything\nlet Anything = cabin.Anything;\nlet _ = cabin.AnythingImplementation;\n\n# Primitives\nlet Text = cabin.Text;\nlet Number = cabin.Number;\n\n# Booleans\nlet Boolean = cabin.Boolean;\nlet true = cabin.true;\nlet false = cabin.false;\n\n# Optionals\nlet Optional = cabin.Optional;\nlet nothing = cabin.Nothing.nothing;\n\n# Results\nlet Attempted = cabin.Attempted;\nlet Error = cabin.Error;\n\n# Collections\nlet List = cabin.List;\nlet Map = cabin.Map;\nlet Object = cabin.Object;\n\n# System\nlet system = cabin.system;\nlet print = system.terminal.print;\nlet input = system.terminal.input;\nlet debug = system.terminal.debug;\nlet TerminalPrintOptions = cabin.TerminalPrintOptions;\nlet TerminalInputOptions = cabin.TerminalInputOptions;\n\n# Compiler\nlet Warning = cabin.Warning;\n","version":0,"languageId":"cabin","uri":"file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/prelude.cabin"}},"jsonrpc":"2.0"}
```

**Handling request:**
```json
{"method":"textDocument/didOpen","params":{"textDocument":{"text":"# The Cabin prelude. This code is placed implicitly at the beginning of all Cabin files, aside from the standard library.\n# It just brings some commonly used names into scope.\n\n# Anything\nlet Anything = cabin.Anything;\nlet _ = cabin.AnythingImplementation;\n\n# Primitives\nlet Text = cabin.Text;\nlet Number = cabin.Number;\n\n# Booleans\nlet Boolean = cabin.Boolean;\nlet true = cabin.true;\nlet false = cabin.false;\n\n# Optionals\nlet Optional = cabin.Optional;\nlet nothing = cabin.Nothing.nothing;\n\n# Results\nlet Attempted = cabin.Attempted;\nlet Error = cabin.Error;\n\n# Collections\nlet List = cabin.List;\nlet Map = cabin.Map;\nlet Object = cabin.Object;\n\n# System\nlet system = cabin.system;\nlet print = system.terminal.print;\nlet input = system.terminal.input;\nlet debug = system.terminal.debug;\nlet TerminalPrintOptions = cabin.TerminalPrintOptions;\nlet TerminalInputOptions = cabin.TerminalInputOptions;\n\n# Compiler\nlet Warning = cabin.Warning;\n","version":0,"languageId":"cabin","uri":"file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/prelude.cabin"}},"jsonrpc":"2.0"}
```

*Opened `file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/prelude.cabin`*

**No response needed.**

# Awaiting next request from client...

**Request received:**
```json
Content-Length: 3743

{"method":"textDocument/didOpen","params":{"textDocument":{"text":"let This = group {};\n\nlet Nothing = either {\n\tnothing\n};\n\nlet Optional = choice<Data> {\n\tData,\n\tNothing\n};\n\nlet Text = group {};\n\nlet Anything = group {\n\tto_text: action(this: Anything): Text\n};\n\nlet AnythingImplementation = extend<T: Anything> T tobe Anything {\n\t#[builtin<\"Anything.to_string\">]\n\tto_text = action(this: Anything): Text\n};\n\nlet Object = group {};\n\nlet Group = group {\n\tfields: Anything,\n};\n\nlet Parameter = group {\n\tname: Text,\n\ttype: Anything\n};\n\nlet OneOf = group {};\n\nlet Function = group {\n\tparameters: Anything,\n\treturn_type: Anything,\n\tcompile_time_parameters: Anything,\n\ttags: Anything,\n\tthis_object: Anything,\n};\n\nlet system_side_effects = new Object {};\n\nlet RuntimeTag = group {\n\treason: Text\n};\n\nlet runtime = action<reason: Text>: RuntimeTag {\n\truntime is new RuntimeTag {\n\t\treason = reason\t\n\t};\n};\n\nlet Field = group {\n\tname: Text,\n\tvalue: Anything,\n};\n\nlet List = group {};\n\nlet Either = group {\n\tvariants: List\n};\n\nlet Boolean = either {\n\ttrue,\n\tfalse\n};\nlet true = Boolean.true; \nlet false = Boolean.false;\n\n# The tag for a built-in function. Functions that are built into the Cabin compiler \n# and run with native code are marked with this, usually via the `builtin<>` function.\nlet BuiltinTag = group {\n\tinternal_name: Text\n};\n\nlet builtin = action<name: Text>: BuiltinTag {\n\tbuiltin is new BuiltinTag {\n\t\tinternal_name = name\n\t};\n};\n\nlet Number = group {\n\n\t#[builtin<\"Number.minus\">]\n\tminus = action(this: Number, other: Number): Number,\n\n\t#[builtin<\"Number.floor\">]\n\tfloor = action(this: Number): Number,\n\n\tto = action(this: Number, end: Number): List {\n\n\t}\n};\n\nlet AddableTo = group<Operand: Anything, Result: Anything> {\n\tplus: action(this: This, other: Operand): Result\n}; \n\ndefault extend Number tobe AddableTo<Number, Number> {\n\n\t#[builtin<\"Number.plus\">]\n\tplus = action(this: Number, other: Number): Number,\n};\n\nlet Error = group {\n\tmessage: Text\n};\n\nlet Attempted = choice<Data> {\n\tData,\n\tError\t\n};\n\nlet TerminalPrintOptions = group {\n\tnewline = true,\n\terror = false,\n};\n\nlet TerminalInputOptions = group {\n\tprompt = \"\"\n};\n\nlet system = new Object {\n\n\tterminal = new Object {\n\n\t\t#[\n\t\t\tbuiltin<\"terminal.print\">, \n\t\t\tsystem_side_effects, \n\t\t\truntime<\"print() is meant for the user. Use debug() at compile-time, or use a run expression to print to the user.\">\n\t\t]\n\t\tprint = action(object: Anything, options: TerminalPrintOptions): Nothing,\n\n\t\t#[\n\t\t\tbuiltin<\"terminal.input\">, \n\t\t\tsystem_side_effects, \n\t\t\truntime<\"Taking input at compile-time can produce varying outputs depending on user input. Consider embedding a file.\">\n\t\t] \n\t\tinput = action(options: TerminalInputOptions): Text,\n\n\t\tdebug = action(object: Anything): Nothing {\n\t\t\tsystem.terminal.print(object);\n\t\t},\n\t},\n};\n\nlet Map = group {\n\tget = action(key: Anything): Anything {},\n\tset = action(key: Anything, value: Anything): Nothing {}\n};\n\nlet Warning = either {\n\n\t# The warning that triggers when an either is created that has zero variants.\n\tEmptyEither,\n\n\t# The warning that triggers when an either is created that has only one variant.\n\tSingleVariantEither,\n\n\t# The warning that triggers when a runtime-preferred function is called at compile-time.\n\tRuntimeFunctionCall,\n};\n\nlet WarningSuppressor = group {\n\twarning: Warning\n};\n","version":0,"languageId":"cabin","uri":"file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/stdlib.cabin"}},"jsonrpc":"2.0"}
```

**Handling request:**
```json
{"method":"textDocument/didOpen","params":{"textDocument":{"text":"let This = group {};\n\nlet Nothing = either {\n\tnothing\n};\n\nlet Optional = choice<Data> {\n\tData,\n\tNothing\n};\n\nlet Text = group {};\n\nlet Anything = group {\n\tto_text: action(this: Anything): Text\n};\n\nlet AnythingImplementation = extend<T: Anything> T tobe Anything {\n\t#[builtin<\"Anything.to_string\">]\n\tto_text = action(this: Anything): Text\n};\n\nlet Object = group {};\n\nlet Group = group {\n\tfields: Anything,\n};\n\nlet Parameter = group {\n\tname: Text,\n\ttype: Anything\n};\n\nlet OneOf = group {};\n\nlet Function = group {\n\tparameters: Anything,\n\treturn_type: Anything,\n\tcompile_time_parameters: Anything,\n\ttags: Anything,\n\tthis_object: Anything,\n};\n\nlet system_side_effects = new Object {};\n\nlet RuntimeTag = group {\n\treason: Text\n};\n\nlet runtime = action<reason: Text>: RuntimeTag {\n\truntime is new RuntimeTag {\n\t\treason = reason\t\n\t};\n};\n\nlet Field = group {\n\tname: Text,\n\tvalue: Anything,\n};\n\nlet List = group {};\n\nlet Either = group {\n\tvariants: List\n};\n\nlet Boolean = either {\n\ttrue,\n\tfalse\n};\nlet true = Boolean.true; \nlet false = Boolean.false;\n\n# The tag for a built-in function. Functions that are built into the Cabin compiler \n# and run with native code are marked with this, usually via the `builtin<>` function.\nlet BuiltinTag = group {\n\tinternal_name: Text\n};\n\nlet builtin = action<name: Text>: BuiltinTag {\n\tbuiltin is new BuiltinTag {\n\t\tinternal_name = name\n\t};\n};\n\nlet Number = group {\n\n\t#[builtin<\"Number.minus\">]\n\tminus = action(this: Number, other: Number): Number,\n\n\t#[builtin<\"Number.floor\">]\n\tfloor = action(this: Number): Number,\n\n\tto = action(this: Number, end: Number): List {\n\n\t}\n};\n\nlet AddableTo = group<Operand: Anything, Result: Anything> {\n\tplus: action(this: This, other: Operand): Result\n}; \n\ndefault extend Number tobe AddableTo<Number, Number> {\n\n\t#[builtin<\"Number.plus\">]\n\tplus = action(this: Number, other: Number): Number,\n};\n\nlet Error = group {\n\tmessage: Text\n};\n\nlet Attempted = choice<Data> {\n\tData,\n\tError\t\n};\n\nlet TerminalPrintOptions = group {\n\tnewline = true,\n\terror = false,\n};\n\nlet TerminalInputOptions = group {\n\tprompt = \"\"\n};\n\nlet system = new Object {\n\n\tterminal = new Object {\n\n\t\t#[\n\t\t\tbuiltin<\"terminal.print\">, \n\t\t\tsystem_side_effects, \n\t\t\truntime<\"print() is meant for the user. Use debug() at compile-time, or use a run expression to print to the user.\">\n\t\t]\n\t\tprint = action(object: Anything, options: TerminalPrintOptions): Nothing,\n\n\t\t#[\n\t\t\tbuiltin<\"terminal.input\">, \n\t\t\tsystem_side_effects, \n\t\t\truntime<\"Taking input at compile-time can produce varying outputs depending on user input. Consider embedding a file.\">\n\t\t] \n\t\tinput = action(options: TerminalInputOptions): Text,\n\n\t\tdebug = action(object: Anything): Nothing {\n\t\t\tsystem.terminal.print(object);\n\t\t},\n\t},\n};\n\nlet Map = group {\n\tget = action(key: Anything): Anything {},\n\tset = action(key: Anything, value: Anything): Nothing {}\n};\n\nlet Warning = either {\n\n\t# The warning that triggers when an either is created that has zero variants.\n\tEmptyEither,\n\n\t# The warning that triggers when an either is created that has only one variant.\n\tSingleVariantEither,\n\n\t# The warning that triggers when a runtime-preferred function is called at compile-time.\n\tRuntimeFunctionCall,\n};\n\nlet WarningSuppressor = group {\n\twarning: Warning\n};\n","version":0,"languageId":"cabin","uri":"file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/stdlib.cabin"}},"jsonrpc":"2.0"}
```

*Opened `file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/stdlib.cabin`*

**No response needed.**

# Awaiting next request from client...

**Request received:**
```json
Content-Length: 182

{"method":"textDocument/didSave","params":{"textDocument":{"uri":"file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/stdlib.cabin"}},"jsonrpc":"2.0"}
```

**Handling request:**
```json
{"method":"textDocument/didSave","params":{"textDocument":{"uri":"file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/stdlib.cabin"}},"jsonrpc":"2.0"}
```

**No response needed.**

# Awaiting next request from client...

**Request received:**
```json
Content-Length: 3744

{"method":"textDocument/didChange","params":{"contentChanges":[{"text":"let This = group {};\n\nlet Nothing = either {\n\tnothing\n};\n\nle Optional = choice<Data> {\n\tData,\n\tNothing\n};\n\nlet Text = group {};\n\nlet Anything = group {\n\tto_text: action(this: Anything): Text\n};\n\nlet AnythingImplementation = extend<T: Anything> T tobe Anything {\n\t#[builtin<\"Anything.to_string\">]\n\tto_text = action(this: Anything): Text\n};\n\nlet Object = group {};\n\nlet Group = group {\n\tfields: Anything,\n};\n\nlet Parameter = group {\n\tname: Text,\n\ttype: Anything\n};\n\nlet OneOf = group {};\n\nlet Function = group {\n\tparameters: Anything,\n\treturn_type: Anything,\n\tcompile_time_parameters: Anything,\n\ttags: Anything,\n\tthis_object: Anything,\n};\n\nlet system_side_effects = new Object {};\n\nlet RuntimeTag = group {\n\treason: Text\n};\n\nlet runtime = action<reason: Text>: RuntimeTag {\n\truntime is new RuntimeTag {\n\t\treason = reason\t\n\t};\n};\n\nlet Field = group {\n\tname: Text,\n\tvalue: Anything,\n};\n\nlet List = group {};\n\nlet Either = group {\n\tvariants: List\n};\n\nlet Boolean = either {\n\ttrue,\n\tfalse\n};\nlet true = Boolean.true; \nlet false = Boolean.false;\n\n# The tag for a built-in function. Functions that are built into the Cabin compiler \n# and run with native code are marked with this, usually via the `builtin<>` function.\nlet BuiltinTag = group {\n\tinternal_name: Text\n};\n\nlet builtin = action<name: Text>: BuiltinTag {\n\tbuiltin is new BuiltinTag {\n\t\tinternal_name = name\n\t};\n};\n\nlet Number = group {\n\n\t#[builtin<\"Number.minus\">]\n\tminus = action(this: Number, other: Number): Number,\n\n\t#[builtin<\"Number.floor\">]\n\tfloor = action(this: Number): Number,\n\n\tto = action(this: Number, end: Number): List {\n\n\t}\n};\n\nlet AddableTo = group<Operand: Anything, Result: Anything> {\n\tplus: action(this: This, other: Operand): Result\n}; \n\ndefault extend Number tobe AddableTo<Number, Number> {\n\n\t#[builtin<\"Number.plus\">]\n\tplus = action(this: Number, other: Number): Number,\n};\n\nlet Error = group {\n\tmessage: Text\n};\n\nlet Attempted = choice<Data> {\n\tData,\n\tError\t\n};\n\nlet TerminalPrintOptions = group {\n\tnewline = true,\n\terror = false,\n};\n\nlet TerminalInputOptions = group {\n\tprompt = \"\"\n};\n\nlet system = new Object {\n\n\tterminal = new Object {\n\n\t\t#[\n\t\t\tbuiltin<\"terminal.print\">, \n\t\t\tsystem_side_effects, \n\t\t\truntime<\"print() is meant for the user. Use debug() at compile-time, or use a run expression to print to the user.\">\n\t\t]\n\t\tprint = action(object: Anything, options: TerminalPrintOptions): Nothing,\n\n\t\t#[\n\t\t\tbuiltin<\"terminal.input\">, \n\t\t\tsystem_side_effects, \n\t\t\truntime<\"Taking input at compile-time can produce varying outputs depending on user input. Consider embedding a file.\">\n\t\t] \n\t\tinput = action(options: TerminalInputOptions): Text,\n\n\t\tdebug = action(object: Anything): Nothing {\n\t\t\tsystem.terminal.print(object);\n\t\t},\n\t},\n};\n\nlet Map = group {\n\tget = action(key: Anything): Anything {},\n\tset = action(key: Anything, value: Anything): Nothing {}\n};\n\nlet Warning = either {\n\n\t# The warning that triggers when an either is created that has zero variants.\n\tEmptyEither,\n\n\t# The warning that triggers when an either is created that has only one variant.\n\tSingleVariantEither,\n\n\t# The warning that triggers when a runtime-preferred function is called at compile-time.\n\tRuntimeFunctionCall,\n};\n\nlet WarningSuppressor = group {\n\twarning: Warning\n};\n"}],"textDocument":{"version":3,"uri":"file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/stdlib.cabin"}},"jsonrpc":"2.0"}
```

**Handling request:**
```json
{"method":"textDocument/didChange","params":{"contentChanges":[{"text":"let This = group {};\n\nlet Nothing = either {\n\tnothing\n};\n\nle Optional = choice<Data> {\n\tData,\n\tNothing\n};\n\nlet Text = group {};\n\nlet Anything = group {\n\tto_text: action(this: Anything): Text\n};\n\nlet AnythingImplementation = extend<T: Anything> T tobe Anything {\n\t#[builtin<\"Anything.to_string\">]\n\tto_text = action(this: Anything): Text\n};\n\nlet Object = group {};\n\nlet Group = group {\n\tfields: Anything,\n};\n\nlet Parameter = group {\n\tname: Text,\n\ttype: Anything\n};\n\nlet OneOf = group {};\n\nlet Function = group {\n\tparameters: Anything,\n\treturn_type: Anything,\n\tcompile_time_parameters: Anything,\n\ttags: Anything,\n\tthis_object: Anything,\n};\n\nlet system_side_effects = new Object {};\n\nlet RuntimeTag = group {\n\treason: Text\n};\n\nlet runtime = action<reason: Text>: RuntimeTag {\n\truntime is new RuntimeTag {\n\t\treason = reason\t\n\t};\n};\n\nlet Field = group {\n\tname: Text,\n\tvalue: Anything,\n};\n\nlet List = group {};\n\nlet Either = group {\n\tvariants: List\n};\n\nlet Boolean = either {\n\ttrue,\n\tfalse\n};\nlet true = Boolean.true; \nlet false = Boolean.false;\n\n# The tag for a built-in function. Functions that are built into the Cabin compiler \n# and run with native code are marked with this, usually via the `builtin<>` function.\nlet BuiltinTag = group {\n\tinternal_name: Text\n};\n\nlet builtin = action<name: Text>: BuiltinTag {\n\tbuiltin is new BuiltinTag {\n\t\tinternal_name = name\n\t};\n};\n\nlet Number = group {\n\n\t#[builtin<\"Number.minus\">]\n\tminus = action(this: Number, other: Number): Number,\n\n\t#[builtin<\"Number.floor\">]\n\tfloor = action(this: Number): Number,\n\n\tto = action(this: Number, end: Number): List {\n\n\t}\n};\n\nlet AddableTo = group<Operand: Anything, Result: Anything> {\n\tplus: action(this: This, other: Operand): Result\n}; \n\ndefault extend Number tobe AddableTo<Number, Number> {\n\n\t#[builtin<\"Number.plus\">]\n\tplus = action(this: Number, other: Number): Number,\n};\n\nlet Error = group {\n\tmessage: Text\n};\n\nlet Attempted = choice<Data> {\n\tData,\n\tError\t\n};\n\nlet TerminalPrintOptions = group {\n\tnewline = true,\n\terror = false,\n};\n\nlet TerminalInputOptions = group {\n\tprompt = \"\"\n};\n\nlet system = new Object {\n\n\tterminal = new Object {\n\n\t\t#[\n\t\t\tbuiltin<\"terminal.print\">, \n\t\t\tsystem_side_effects, \n\t\t\truntime<\"print() is meant for the user. Use debug() at compile-time, or use a run expression to print to the user.\">\n\t\t]\n\t\tprint = action(object: Anything, options: TerminalPrintOptions): Nothing,\n\n\t\t#[\n\t\t\tbuiltin<\"terminal.input\">, \n\t\t\tsystem_side_effects, \n\t\t\truntime<\"Taking input at compile-time can produce varying outputs depending on user input. Consider embedding a file.\">\n\t\t] \n\t\tinput = action(options: TerminalInputOptions): Text,\n\n\t\tdebug = action(object: Anything): Nothing {\n\t\t\tsystem.terminal.print(object);\n\t\t},\n\t},\n};\n\nlet Map = group {\n\tget = action(key: Anything): Anything {},\n\tset = action(key: Anything, value: Anything): Nothing {}\n};\n\nlet Warning = either {\n\n\t# The warning that triggers when an either is created that has zero variants.\n\tEmptyEither,\n\n\t# The warning that triggers when an either is created that has only one variant.\n\tSingleVariantEither,\n\n\t# The warning that triggers when a runtime-preferred function is called at compile-time.\n\tRuntimeFunctionCall,\n};\n\nlet WarningSuppressor = group {\n\twarning: Warning\n};\n"}],"textDocument":{"version":3,"uri":"file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/stdlib.cabin"}},"jsonrpc":"2.0"}
```

*Changed `file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/stdlib.cabin`*

**Response:**
```json
Content-Length: 409

{"id":null,"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":"file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/stdlib.cabin","diagnostics":[{"range":{"start":{"line":6,"character":0},"end":{"line":6,"character":2}},"severity":1,"source":"Cabin Language Server","message":"Parse error: Unexpected token: Expected Keyword Let but found Identifier"}]}}
```

# Awaiting next request from client...

**Request received:**
```json
Content-Length: 182

{"method":"textDocument/didSave","params":{"textDocument":{"uri":"file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/stdlib.cabin"}},"jsonrpc":"2.0"}
```

**Handling request:**
```json
{"method":"textDocument/didSave","params":{"textDocument":{"uri":"file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/stdlib.cabin"}},"jsonrpc":"2.0"}
```

**No response needed.**

# Awaiting next request from client...

**Request received:**
```json
Content-Length: 3745

{"method":"textDocument/didChange","params":{"contentChanges":[{"text":"let This = group {};\n\nlet Nothing = either {\n\tnothing\n};\n\nlet Optional = choice<Data> {\n\tData,\n\tNothing\n};\n\nlet Text = group {};\n\nlet Anything = group {\n\tto_text: action(this: Anything): Text\n};\n\nlet AnythingImplementation = extend<T: Anything> T tobe Anything {\n\t#[builtin<\"Anything.to_string\">]\n\tto_text = action(this: Anything): Text\n};\n\nlet Object = group {};\n\nlet Group = group {\n\tfields: Anything,\n};\n\nlet Parameter = group {\n\tname: Text,\n\ttype: Anything\n};\n\nlet OneOf = group {};\n\nlet Function = group {\n\tparameters: Anything,\n\treturn_type: Anything,\n\tcompile_time_parameters: Anything,\n\ttags: Anything,\n\tthis_object: Anything,\n};\n\nlet system_side_effects = new Object {};\n\nlet RuntimeTag = group {\n\treason: Text\n};\n\nlet runtime = action<reason: Text>: RuntimeTag {\n\truntime is new RuntimeTag {\n\t\treason = reason\t\n\t};\n};\n\nlet Field = group {\n\tname: Text,\n\tvalue: Anything,\n};\n\nlet List = group {};\n\nlet Either = group {\n\tvariants: List\n};\n\nlet Boolean = either {\n\ttrue,\n\tfalse\n};\nlet true = Boolean.true; \nlet false = Boolean.false;\n\n# The tag for a built-in function. Functions that are built into the Cabin compiler \n# and run with native code are marked with this, usually via the `builtin<>` function.\nlet BuiltinTag = group {\n\tinternal_name: Text\n};\n\nlet builtin = action<name: Text>: BuiltinTag {\n\tbuiltin is new BuiltinTag {\n\t\tinternal_name = name\n\t};\n};\n\nlet Number = group {\n\n\t#[builtin<\"Number.minus\">]\n\tminus = action(this: Number, other: Number): Number,\n\n\t#[builtin<\"Number.floor\">]\n\tfloor = action(this: Number): Number,\n\n\tto = action(this: Number, end: Number): List {\n\n\t}\n};\n\nlet AddableTo = group<Operand: Anything, Result: Anything> {\n\tplus: action(this: This, other: Operand): Result\n}; \n\ndefault extend Number tobe AddableTo<Number, Number> {\n\n\t#[builtin<\"Number.plus\">]\n\tplus = action(this: Number, other: Number): Number,\n};\n\nlet Error = group {\n\tmessage: Text\n};\n\nlet Attempted = choice<Data> {\n\tData,\n\tError\t\n};\n\nlet TerminalPrintOptions = group {\n\tnewline = true,\n\terror = false,\n};\n\nlet TerminalInputOptions = group {\n\tprompt = \"\"\n};\n\nlet system = new Object {\n\n\tterminal = new Object {\n\n\t\t#[\n\t\t\tbuiltin<\"terminal.print\">, \n\t\t\tsystem_side_effects, \n\t\t\truntime<\"print() is meant for the user. Use debug() at compile-time, or use a run expression to print to the user.\">\n\t\t]\n\t\tprint = action(object: Anything, options: TerminalPrintOptions): Nothing,\n\n\t\t#[\n\t\t\tbuiltin<\"terminal.input\">, \n\t\t\tsystem_side_effects, \n\t\t\truntime<\"Taking input at compile-time can produce varying outputs depending on user input. Consider embedding a file.\">\n\t\t] \n\t\tinput = action(options: TerminalInputOptions): Text,\n\n\t\tdebug = action(object: Anything): Nothing {\n\t\t\tsystem.terminal.print(object);\n\t\t},\n\t},\n};\n\nlet Map = group {\n\tget = action(key: Anything): Anything {},\n\tset = action(key: Anything, value: Anything): Nothing {}\n};\n\nlet Warning = either {\n\n\t# The warning that triggers when an either is created that has zero variants.\n\tEmptyEither,\n\n\t# The warning that triggers when an either is created that has only one variant.\n\tSingleVariantEither,\n\n\t# The warning that triggers when a runtime-preferred function is called at compile-time.\n\tRuntimeFunctionCall,\n};\n\nlet WarningSuppressor = group {\n\twarning: Warning\n};\n"}],"textDocument":{"version":5,"uri":"file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/stdlib.cabin"}},"jsonrpc":"2.0"}
```

**Handling request:**
```json
{"method":"textDocument/didChange","params":{"contentChanges":[{"text":"let This = group {};\n\nlet Nothing = either {\n\tnothing\n};\n\nlet Optional = choice<Data> {\n\tData,\n\tNothing\n};\n\nlet Text = group {};\n\nlet Anything = group {\n\tto_text: action(this: Anything): Text\n};\n\nlet AnythingImplementation = extend<T: Anything> T tobe Anything {\n\t#[builtin<\"Anything.to_string\">]\n\tto_text = action(this: Anything): Text\n};\n\nlet Object = group {};\n\nlet Group = group {\n\tfields: Anything,\n};\n\nlet Parameter = group {\n\tname: Text,\n\ttype: Anything\n};\n\nlet OneOf = group {};\n\nlet Function = group {\n\tparameters: Anything,\n\treturn_type: Anything,\n\tcompile_time_parameters: Anything,\n\ttags: Anything,\n\tthis_object: Anything,\n};\n\nlet system_side_effects = new Object {};\n\nlet RuntimeTag = group {\n\treason: Text\n};\n\nlet runtime = action<reason: Text>: RuntimeTag {\n\truntime is new RuntimeTag {\n\t\treason = reason\t\n\t};\n};\n\nlet Field = group {\n\tname: Text,\n\tvalue: Anything,\n};\n\nlet List = group {};\n\nlet Either = group {\n\tvariants: List\n};\n\nlet Boolean = either {\n\ttrue,\n\tfalse\n};\nlet true = Boolean.true; \nlet false = Boolean.false;\n\n# The tag for a built-in function. Functions that are built into the Cabin compiler \n# and run with native code are marked with this, usually via the `builtin<>` function.\nlet BuiltinTag = group {\n\tinternal_name: Text\n};\n\nlet builtin = action<name: Text>: BuiltinTag {\n\tbuiltin is new BuiltinTag {\n\t\tinternal_name = name\n\t};\n};\n\nlet Number = group {\n\n\t#[builtin<\"Number.minus\">]\n\tminus = action(this: Number, other: Number): Number,\n\n\t#[builtin<\"Number.floor\">]\n\tfloor = action(this: Number): Number,\n\n\tto = action(this: Number, end: Number): List {\n\n\t}\n};\n\nlet AddableTo = group<Operand: Anything, Result: Anything> {\n\tplus: action(this: This, other: Operand): Result\n}; \n\ndefault extend Number tobe AddableTo<Number, Number> {\n\n\t#[builtin<\"Number.plus\">]\n\tplus = action(this: Number, other: Number): Number,\n};\n\nlet Error = group {\n\tmessage: Text\n};\n\nlet Attempted = choice<Data> {\n\tData,\n\tError\t\n};\n\nlet TerminalPrintOptions = group {\n\tnewline = true,\n\terror = false,\n};\n\nlet TerminalInputOptions = group {\n\tprompt = \"\"\n};\n\nlet system = new Object {\n\n\tterminal = new Object {\n\n\t\t#[\n\t\t\tbuiltin<\"terminal.print\">, \n\t\t\tsystem_side_effects, \n\t\t\truntime<\"print() is meant for the user. Use debug() at compile-time, or use a run expression to print to the user.\">\n\t\t]\n\t\tprint = action(object: Anything, options: TerminalPrintOptions): Nothing,\n\n\t\t#[\n\t\t\tbuiltin<\"terminal.input\">, \n\t\t\tsystem_side_effects, \n\t\t\truntime<\"Taking input at compile-time can produce varying outputs depending on user input. Consider embedding a file.\">\n\t\t] \n\t\tinput = action(options: TerminalInputOptions): Text,\n\n\t\tdebug = action(object: Anything): Nothing {\n\t\t\tsystem.terminal.print(object);\n\t\t},\n\t},\n};\n\nlet Map = group {\n\tget = action(key: Anything): Anything {},\n\tset = action(key: Anything, value: Anything): Nothing {}\n};\n\nlet Warning = either {\n\n\t# The warning that triggers when an either is created that has zero variants.\n\tEmptyEither,\n\n\t# The warning that triggers when an either is created that has only one variant.\n\tSingleVariantEither,\n\n\t# The warning that triggers when a runtime-preferred function is called at compile-time.\n\tRuntimeFunctionCall,\n};\n\nlet WarningSuppressor = group {\n\twarning: Warning\n};\n"}],"textDocument":{"version":5,"uri":"file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/stdlib.cabin"}},"jsonrpc":"2.0"}
```

*Changed `file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/stdlib.cabin`*

**Response:**
```json
Content-Length: 418

{"id":null,"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":"file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/stdlib.cabin","diagnostics":[{"range":{"start":{"line":102,"character":0},"end":{"line":102,"character":7}},"severity":1,"source":"Cabin Language Server","message":"Parse error: Unexpected token: Expected Keyword Let but found Keyword Default"}]}}
```

# Awaiting next request from client...

**Request received:**
```json
Content-Length: 182

{"method":"textDocument/didSave","params":{"textDocument":{"uri":"file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/stdlib.cabin"}},"jsonrpc":"2.0"}
```

**Handling request:**
```json
{"method":"textDocument/didSave","params":{"textDocument":{"uri":"file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/stdlib.cabin"}},"jsonrpc":"2.0"}
```

**No response needed.**

# Awaiting next request from client...

**Request received:**
```json
Content-Length: 182

{"method":"textDocument/didSave","params":{"textDocument":{"uri":"file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/stdlib.cabin"}},"jsonrpc":"2.0"}
```

**Handling request:**
```json
{"method":"textDocument/didSave","params":{"textDocument":{"uri":"file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/stdlib.cabin"}},"jsonrpc":"2.0"}
```

**No response needed.**

# Awaiting next request from client...

**Request received:**
```json
Content-Length: 225

{"method":"textDocument/hover","params":{"position":{"character":16,"line":28},"textDocument":{"uri":"file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/stdlib.cabin"}},"jsonrpc":"2.0","id":2}
```

**Handling request:**
```json
{"method":"textDocument/hover","params":{"position":{"character":16,"line":28},"textDocument":{"uri":"file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/stdlib.cabin"}},"jsonrpc":"2.0","id":2}
```

**Response:**
```json
Content-Length: 603

{"id":2,"jsonrpc":"2.0","result":{"contents":"`group`\n\n---\n\n`group` is used to declare a group type, similar to a `struct` in other languages:\n\n```cabin\nlet Person = group {\n\tname: Text,\n\tage: Number\n};\n\nlet john = new Person {\n\tname = \"John\",\n\tage = 30\n};\n```\n\nGroups are nominally typed, meaning even if two groups share the same structure,\nyou cannot use them interchangeably, i.e., the following isn't valid:\n\n```cabin\nlet Point = group { x: Number, y: Number };\nlet Position = group { x: Number, y: Number };\n\nlet x: Point = new Position { x = 10, y = 10 };\n```\n"}}
```

# Awaiting next request from client...

**Request received:**
```json
Content-Length: 225

{"method":"textDocument/hover","params":{"position":{"character":16,"line":28},"textDocument":{"uri":"file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/stdlib.cabin"}},"jsonrpc":"2.0","id":3}
```

**Handling request:**
```json
{"method":"textDocument/hover","params":{"position":{"character":16,"line":28},"textDocument":{"uri":"file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/stdlib.cabin"}},"jsonrpc":"2.0","id":3}
```

**Response:**
```json
Content-Length: 603

{"id":3,"jsonrpc":"2.0","result":{"contents":"`group`\n\n---\n\n`group` is used to declare a group type, similar to a `struct` in other languages:\n\n```cabin\nlet Person = group {\n\tname: Text,\n\tage: Number\n};\n\nlet john = new Person {\n\tname = \"John\",\n\tage = 30\n};\n```\n\nGroups are nominally typed, meaning even if two groups share the same structure,\nyou cannot use them interchangeably, i.e., the following isn't valid:\n\n```cabin\nlet Point = group { x: Number, y: Number };\nlet Position = group { x: Number, y: Number };\n\nlet x: Point = new Position { x = 10, y = 10 };\n```\n"}}
```

# Awaiting next request from client...

**Request received:**
```json
Content-Length: 182

{"method":"textDocument/didSave","params":{"textDocument":{"uri":"file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/stdlib.cabin"}},"jsonrpc":"2.0"}
```

**Handling request:**
```json
{"method":"textDocument/didSave","params":{"textDocument":{"uri":"file:///home/violet/Documents/Coding/Developer%20Tools/Cabin/cabin/crates/cabin/std/stdlib.cabin"}},"jsonrpc":"2.0"}
```

**No response needed.**

# Awaiting next request from client...

**Request received:**
```json
Content-Length: 44

{"method":"shutdown","jsonrpc":"2.0","id":4}
```

**Handling request:**
```json
{"method":"shutdown","jsonrpc":"2.0","id":4}
```

ERROR: unknown variant `shutdown`, expected one of `initialize`, `initialized`, `textDocument/didOpen`, `textDocument/didChange`, `textDocument/hover`, `textDocument/didSave` at line 1 column 44
Shutting down language server.