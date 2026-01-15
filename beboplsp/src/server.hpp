#pragma once

#include "workspace.hpp"
#include <lsp/connection.h>
#include <lsp/messagehandler.h>
#include <lsp/messages.h>
#include <string>
#include <vector>

namespace beboplsp {

class Server {
public:
    Server(lsp::io::Stream& io, std::vector<std::string> includePaths);

    void run();
    void stop() { running_ = false; }

private:
    void registerHandlers();
    void publishDiagnostics(const lsp::DocumentUri& uri);

    lsp::requests::Initialize::Result onInitialize(lsp::requests::Initialize::Params&& params);
    void onInitialized(lsp::notifications::Initialized::Params&& params);
    lsp::requests::Shutdown::Result onShutdown();
    void onExit();

    void onDidOpen(lsp::notifications::TextDocument_DidOpen::Params&& params);
    void onDidChange(lsp::notifications::TextDocument_DidChange::Params&& params);
    void onDidClose(lsp::notifications::TextDocument_DidClose::Params&& params);
    void onDidSave(lsp::notifications::TextDocument_DidSave::Params&& params);

    lsp::requests::TextDocument_Hover::Result onHover(
        lsp::requests::TextDocument_Hover::Params&& params
    );
    lsp::requests::TextDocument_Definition::Result onDefinition(
        lsp::requests::TextDocument_Definition::Params&& params
    );
    lsp::requests::TextDocument_References::Result onReferences(
        lsp::requests::TextDocument_References::Params&& params
    );
    lsp::requests::TextDocument_DocumentSymbol::Result onDocumentSymbol(
        lsp::requests::TextDocument_DocumentSymbol::Params&& params
    );
    lsp::requests::TextDocument_Formatting::Result onFormatting(
        lsp::requests::TextDocument_Formatting::Params&& params
    );
    lsp::requests::TextDocument_Completion::Result onCompletion(
        lsp::requests::TextDocument_Completion::Params&& params
    );
    lsp::requests::TextDocument_CodeAction::Result onCodeAction(
        lsp::requests::TextDocument_CodeAction::Params&& params
    );
    lsp::requests::TextDocument_RangeFormatting::Result onRangeFormatting(
        lsp::requests::TextDocument_RangeFormatting::Params&& params
    );
    lsp::requests::TextDocument_DocumentHighlight::Result onDocumentHighlight(
        lsp::requests::TextDocument_DocumentHighlight::Params&& params
    );
    lsp::requests::TextDocument_FoldingRange::Result onFoldingRange(
        lsp::requests::TextDocument_FoldingRange::Params&& params
    );
    lsp::requests::TextDocument_DocumentLink::Result onDocumentLink(
        lsp::requests::TextDocument_DocumentLink::Params&& params
    );
    lsp::requests::Workspace_Symbol::Result onWorkspaceSymbol(
        lsp::requests::Workspace_Symbol::Params&& params
    );
    lsp::requests::TextDocument_Rename::Result onRename(
        lsp::requests::TextDocument_Rename::Params&& params
    );
    lsp::requests::TextDocument_PrepareRename::Result onPrepareRename(
        lsp::requests::TextDocument_PrepareRename::Params&& params
    );

    std::string formatType(const bebop_type_t* type, bebop_context_t* ctx = nullptr);
    std::string formatDef(const bebop_def_t* def, bebop_context_t* ctx = nullptr);
    std::string formatBranch(const bebop_union_branch_t* branch, bebop_context_t* ctx = nullptr);
    std::string formatMember(const bebop_enum_member_t* member);
    std::string formatMethod(const bebop_method_t* method, bebop_context_t* ctx = nullptr);
    std::string formatDecoratorDef(const bebop_def_t* def);
    lsp::SymbolKind defKindToSymbolKind(bebop_def_kind_t kind);
    lsp::CompletionItemKind defKindToCompletionKind(bebop_def_kind_t kind);

    lsp::Connection connection_;
    lsp::MessageHandler handler_;
    Workspace workspace_;
    bool running_ = false;
    bool initialized_ = false;
};

} // namespace beboplsp
