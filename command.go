package main

import (
	"fmt"
	"os"
	"os/exec"

	"github.com/fatih/color"
	"github.com/spf13/cobra"
)

var StartCommand = &cobra.Command{
	Use:   "start",
	Short: "Start csync as daemon",

	Args: cobra.ExactArgs(0),

	RunE: func(_ *cobra.Command, _ []string) error {
		d, err := NewDaemon()
		if err != nil {
			return err
		}

		return d.Start(func() error {
			clip, err := NewClipboard()
			if err != nil {
				return err
			}
			clip.Run()
			return nil
		})
	},
}

var StopCommand = &cobra.Command{
	Use:   "stop",
	Short: "Stop csync daemon",

	Args: cobra.ExactArgs(0),

	RunE: func(_ *cobra.Command, _ []string) error {
		d, err := NewDaemon()
		if err != nil {
			return err
		}
		return d.Stop()
	},
}

var StatusCommand = &cobra.Command{
	Use:   "status",
	Short: "Show csync daemon status",

	Args: cobra.ExactArgs(0),

	RunE: func(_ *cobra.Command, _ []string) error {
		d, err := NewDaemon()
		if err != nil {
			return err
		}
		return d.ShowStatus()
	},
}

var LogsCommand = &cobra.Command{
	Use:   "logs",
	Short: "Show csync daemon logs",

	DisableFlagParsing: true,

	RunE: func(_ *cobra.Command, args []string) error {
		d, err := NewDaemon()
		if err != nil {
			return err
		}
		path := d.LogPath()
		args = append(args, path)
		cmd := exec.Command("tail", args...)
		cmd.Stdout = os.Stdout
		cmd.Stderr = os.Stderr
		cmd.Stdin = os.Stdin
		return cmd.Run()
	},
}

var (
	sendFilename string
	sendImage    string
)

var SendCommand = &cobra.Command{
	Use:   "send [-f filename] [-i image] [message]",
	Short: "Send message or image to remote",

	Args: cobra.MaximumNArgs(1),

	RunE: func(_ *cobra.Command, args []string) error {
		redis, err := NewRedis()
		if err != nil {
			return err
		}

		if sendImage != "" {
			data, err := os.ReadFile(sendImage)
			if err != nil {
				return fmt.Errorf("Read image: %w", err)
			}

			frame := NewDataFrame(DataFrameImage, data)
			err = redis.Send(frame)
			if err != nil {
				return fmt.Errorf("Send image to redis: %w", err)
			}
		}

		var sendText []byte
		if len(args) == 1 {
			sendText = []byte(args[0])
		}
		if sendFilename != "" {
			sendText, err = os.ReadFile(sendFilename)
			if err != nil {
				return fmt.Errorf("Read text: %w", err)
			}
		}
		if len(sendText) > 0 {
			frame := NewDataFrame(DataFrameText, sendText)
			err = redis.Send(frame)
			if err != nil {
				return fmt.Errorf("Send text to redis: %w", err)
			}
		}

		return nil
	},
}

func init() {
	SendCommand.PersistentFlags().StringVarP(&sendFilename, "file", "f", "", "Read text file to send")
	SendCommand.PersistentFlags().StringVarP(&sendImage, "image", "i", "", "Read image file to send")
}

func ErrorExit(err error) {
	fmt.Printf("%s: %v\n", color.RedString("error"), err)
	os.Exit(1)
}
