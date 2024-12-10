package main

import (
	"github.com/spf13/cobra"
)

// TODO ... ECC import

func init() {
	// TODO ...
}

var gkrProofMergeCmd = &cobra.Command{
	Use:   "merge",
	Short: "Merge multiple Expander GKR proofs into one",
	Long: `
Merge multiple Expander GKR proofs into one
over same circuit, same field, same PCS`,
	Args: cobra.NoArgs,
	Run: func(cmd *cobra.Command, args []string) {
		cmd.HelpFunc()(cmd, args)
	},
}
