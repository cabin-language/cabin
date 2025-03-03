package tree_sitter_cabin_test

import (
	"testing"

	tree_sitter "github.com/tree-sitter/go-tree-sitter"
	tree_sitter_cabin "github.com/language-cabin/tree-sitter-cabin/bindings/go"
)

func TestCanLoadGrammar(t *testing.T) {
	language := tree_sitter.NewLanguage(tree_sitter_cabin.Language())
	if language == nil {
		t.Errorf("Error loading Cabin grammar")
	}
}
