package main

import "github.com/spf13/cobra"

var Command = &cobra.Command{
	Use:   "csync",
	Short: "Start csync",

	Args: cobra.ExactArgs(0),

	RunE: func(_ *cobra.Command, _ []string) error {
		clip, err := NewClipboard()
		if err != nil {
			return err
		}
		clip.Run()
		return nil
	},
}

func main() {
	Command.AddCommand(StartCommand, StopCommand, StatusCommand, LogsCommand)

	err := Command.Execute()
	if err != nil {
		ErrorExit(err)
	}
}
