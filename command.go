package main

import (
	"bytes"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path"
	"path/filepath"
	"runtime"
	"strings"
	"time"

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

var (
	recvFilename string
	recvAppend   bool
	recvImage    string
)

var RecvCommand = &cobra.Command{
	Use:   "recv [-f filename] [-i image] [-a]",
	Short: "Receive data and write to file rather than clipboard",

	Args: cobra.ExactArgs(0),

	RunE: func(_ *cobra.Command, _ []string) error {
		var err error
		if recvFilename != "" {
			err = ensureDir(recvFilename)
			if err != nil {
				return err
			}
		}

		if recvImage != "" {
			err = ensureDir(recvImage)
			if err != nil {
				return err
			}
		}

		redis, err := NewRedis()
		if err != nil {
			return err
		}

		for frame := range redis.Sub() {
			switch frame.Type {
			case DataFrameImage:
				if recvImage == "" {
					frame.LogEntry.Info("Received image")
					continue
				}

				start := time.Now()
				err = os.WriteFile(recvImage, frame.Data, 0644)
				if err != nil {
					frame.LogEntry.Errorf("Write image data to file error: %v", err)
					continue
				}

				frame.LogEntry.Infof("Write image data to file %s done, took %v", recvImage, time.Since(start))

			case DataFrameText:
				if recvFilename == "" {
					text := string(frame.Data)
					text = strings.ReplaceAll(text, "\n", "\\n")
					frame.LogEntry.Info(text)
					continue
				}

				var flag int
				if recvAppend {
					flag = os.O_CREATE | os.O_APPEND | os.O_WRONLY
				} else {
					flag = os.O_CREATE | os.O_TRUNC | os.O_WRONLY
				}

				file, err := os.OpenFile(recvFilename, flag, 0644)
				if err != nil {
					frame.LogEntry.Errorf("Open text file error: %v", err)
					continue
				}

				data := frame.Data
				if recvAppend {
					header := fmt.Sprintf(">>>> From: %s, Time: %s", frame.From,
						time.Now().Format("2006-01-02 15:04:05"))
					content := fmt.Sprintf("%s\n%s\n\n", header, string(frame.Data))

					data = []byte(content)
				}

				buf := bytes.NewBuffer(data)
				_, err = io.Copy(file, buf)
				if err != nil {
					frame.LogEntry.Errorf("Write text to file error: %v", err)
					continue
				}

				err = file.Close()
				if err != nil {
					frame.LogEntry.Errorf("Close text file error: %v", err)
					continue
				}

			}
		}

		return nil
	},
}

var UpdateCommand = &cobra.Command{
	Use:   "update [version]",
	Short: "Execute self-update",

	Args: cobra.MaximumNArgs(1),

	RunE: func(_ *cobra.Command, args []string) error {
		targetVersion := ""
		if len(args) >= 1 {
			targetVersion = args[0]
		}

		var err error
		if targetVersion == "" {
			fmt.Println("Checking new version for csync")
			targetVersion, err = GetLatestVersion()
			if err != nil {
				return fmt.Errorf("Get latest release from github: %w", err)
			}
		}

		if Version == targetVersion {
			fmt.Println("Your csync is up-to-date")
			return nil
		}

		fmt.Printf("Do you want to update csync to %s? (y/n) ", color.MagentaString(targetVersion))
		var confirm string
		fmt.Scanf("%s", &confirm)
		if confirm != "y" {
			os.Exit(1)
		}

		tmpDir := os.TempDir()
		tarPath := filepath.Join(tmpDir, "csync-update", "csync.tar.gz")
		target := fmt.Sprintf("%s-%s", runtime.GOOS, runtime.GOARCH)
		fmt.Printf("Downloading csync(version:%s, target:%s) to %s\n", targetVersion, target, tarPath)
		err = DownloadRelease(targetVersion, target, tarPath)
		if err != nil {
			return fmt.Errorf("Download github release: %w", err)
		}

		binPath := filepath.Join(tmpDir, "csync-update", "bin")
		fmt.Printf("Untaring csync binary to %s\n", binPath)
		err = UnTarTo(tarPath, binPath)
		if err != nil {
			return fmt.Errorf("Untar tar.gz file: %w", err)
		}

		binPath = filepath.Join(binPath, "csync")
		currentBinPath, err := os.Executable()
		if err != nil {
			return fmt.Errorf("Get current executable file: %w", err)
		}

		fmt.Printf("Replacing binary %s\n", currentBinPath)

		// FIXME: Support windows
		var buf bytes.Buffer
		cmd := exec.Command("mv", binPath, currentBinPath)
		cmd.Stdout = &buf
		cmd.Stderr = &buf
		err = cmd.Run()
		if err != nil {
			return fmt.Errorf("Move binary file error: %s", buf.String())
		}

		fmt.Printf("Update csync to %s done\n", targetVersion)

		toRemoveDir := filepath.Join(tmpDir, "csync-update")
		err = os.RemoveAll(toRemoveDir)
		if err != nil {
			return fmt.Errorf("Remove update tmp dir: %w", err)
		}

		return nil
	},
}

func init() {
	SendCommand.PersistentFlags().StringVarP(&sendFilename, "file", "f", "", "Read text file to send")
	SendCommand.PersistentFlags().StringVarP(&sendImage, "image", "i", "", "Read image file to send")

	RecvCommand.PersistentFlags().StringVarP(&recvFilename, "file", "f", "", "Write received text to this file")
	RecvCommand.PersistentFlags().BoolVarP(&recvAppend, "append", "a", false, "Append text file, no overwrite")
	RecvCommand.PersistentFlags().StringVarP(&recvImage, "image", "i", "", "Write received image to this file")
}

func ErrorExit(err error) {
	fmt.Printf("%s: %v\n", color.RedString("error"), err)
	os.Exit(1)
}

func ensureDir(filepath string) error {
	dir := path.Dir(filepath)
	stat, err := os.Stat(dir)
	if err != nil {
		if os.IsNotExist(err) {
			err = os.MkdirAll(dir, os.ModePerm)
			if err != nil {
				return err
			}
		}
		return err
	}
	if !stat.IsDir() {
		return fmt.Errorf("Path %s is not a directory", dir)
	}

	return nil
}
