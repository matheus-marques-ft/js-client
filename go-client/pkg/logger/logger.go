package logger

import (
	"os"
	"path/filepath"
	"time"

	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
)

const (
	logTmFmtWithMS = "2006-01-02 15:04:05.000"
)

func InitLogger() *zap.Logger {
	writeSyncer := getLogWriter()
	encoder := getEncoder()
	core := zapcore.NewCore(encoder, writeSyncer, zapcore.DebugLevel)
	return zap.New(core, zap.AddCaller())
}

// Custom time output format
func customTimeEncoder(t time.Time, enc zapcore.PrimitiveArrayEncoder) {
	enc.AppendString("[" + t.Format(logTmFmtWithMS) + "]")
}

// Custom log level display
func customLevelEncoder(level zapcore.Level, enc zapcore.PrimitiveArrayEncoder) {
	enc.AppendString("[" + level.CapitalString() + "]")
}

// Custom file:line output item
func customCallerEncoder(caller zapcore.EntryCaller, enc zapcore.PrimitiveArrayEncoder) {
	enc.AppendString("[" + caller.TrimmedPath() + "]")
}

func getEncoder() zapcore.Encoder {
	encoderConfig := zap.NewProductionEncoderConfig()
	encoderConfig.EncodeTime = customTimeEncoder     // Custom time format
	encoderConfig.EncodeLevel = customLevelEncoder   // Lowercase encoder
	encoderConfig.EncodeCaller = customCallerEncoder // Full-path encoder
	return zapcore.NewConsoleEncoder(encoderConfig)
}

func getLogWriter() zapcore.WriteSyncer {
	dir, _ := os.UserConfigDir()
	logDir := filepath.Join(dir, "jumpserver-client")
	// Make sure the log directory exists
	if err := os.MkdirAll(logDir, 0755); err != nil {
		// Fall back to stdout if creating the directory fails
		return zapcore.AddSync(os.Stdout)
	}
	filePath := filepath.Join(logDir, "client.log")
	file, err := os.Create(filePath)
	if err != nil {
		// Fall back to stdout if creating the file fails
		return zapcore.AddSync(os.Stdout)
	}
	return zapcore.AddSync(file)
}
