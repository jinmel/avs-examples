package handlers

import (
	"context"
	"errors"
	"fmt"
	"math"
	"strings"
	"unicode"

	"github.com/sashabaranov/go-openai"
)

// Message represents a chat message with a role and content
type Message struct {
	Role    string
	Content string
}

// ChatResponse contains the input prompt and the response from the AI
type ChatResponse struct {
	InputPrompt string
	Response    string
}

// Agent defines the interface for chat agents
type Agent interface {
	SetPrompt(prompt string) Agent
	Chat(messages []Message) (ChatResponse, error)
	Prompt() string
}

// OpenAIAgent implements the Agent interface for OpenAI
type OpenAIAgent struct {
	client      *openai.Client
	model       string
	temperature float32
	prompt      string
}

// NewOpenAIAgent creates a new OpenAI agent
func NewOpenAIAgent(apiKey string, model string, temperature float32) *OpenAIAgent {
	client := openai.NewClient(apiKey)

	return &OpenAIAgent{
		client:      client,
		model:       model,
		temperature: temperature,
		prompt:      "",
	}
}

// SetPrompt sets the system prompt for the agent
func (a *OpenAIAgent) SetPrompt(prompt string) Agent {
	a.prompt = prompt
	return a
}

// Prompt returns the current system prompt
func (a *OpenAIAgent) Prompt() string {
	return a.prompt
}

// Chat sends messages to OpenAI and returns the response
func (a *OpenAIAgent) Chat(messages []Message) (ChatResponse, error) {
	fmt.Println("Sending the following messages to OpenAI:")

	// Collect all message contents for the input prompt
	var inputPromptParts []string
	for i, msg := range messages {
		fmt.Printf("  Message %d: role=%s, content=%s\n", i, msg.Role, msg.Content)
		inputPromptParts = append(inputPromptParts, fmt.Sprintf("%s:\n%s", msg.Role, msg.Content))
	}
	inputPrompt := strings.Join(inputPromptParts, "\n\n")

	// Convert our Message type to the library's ChatCompletionMessage type
	var requestMessages []openai.ChatCompletionMessage
	for _, msg := range messages {
		chatMsg := openai.ChatCompletionMessage{
			Content: msg.Content,
		}

		switch msg.Role {
		case "system":
			chatMsg.Role = openai.ChatMessageRoleSystem
		case "assistant":
			chatMsg.Role = openai.ChatMessageRoleAssistant
		default:
			chatMsg.Role = openai.ChatMessageRoleUser
		}

		requestMessages = append(requestMessages, chatMsg)
	}

	// Create the request
	request := openai.ChatCompletionRequest{
		Model:    a.model,
		Messages: requestMessages,
	}

	// Send the request
	response, err := a.client.CreateChatCompletion(context.Background(), request)
	if err != nil {
		return ChatResponse{}, err
	}

	fmt.Printf("Response: %+v\n", response)

	// Extract the response content as plain string
	if len(response.Choices) == 0 {
		return ChatResponse{}, errors.New("no completion choices returned")
	}

	// Return the plain string response
	return ChatResponse{
		InputPrompt: inputPrompt,
		Response:    response.Choices[0].Message.Content,
	}, nil
}

// CalculateStringSimilarity calculates the similarity between two strings
// Returns a value between 0 (completely different) and 1 (identical)
func CalculateStringSimilarity(str1, str2 string) float64 {
	// Normalize strings: convert to lowercase and remove punctuation
	str1 = normalizeString(str1)
	str2 = normalizeString(str2)

	// If either string is empty, return appropriate similarity
	if len(str1) == 0 && len(str2) == 0 {
		return 1.0 // Both empty means they're identical
	}
	if len(str1) == 0 || len(str2) == 0 {
		return 0.0 // One empty means completely different
	}

	// Calculate Levenshtein distance
	distance := levenshteinDistance(str1, str2)

	// Convert distance to similarity score (0 to 1)
	maxLen := math.Max(float64(len(str1)), float64(len(str2)))
	similarity := 1.0 - (float64(distance) / maxLen)

	return similarity
}

// normalizeString converts a string to lowercase and removes punctuation
func normalizeString(s string) string {
	// Convert to lowercase
	s = strings.ToLower(s)

	// Remove punctuation and extra whitespace
	var result strings.Builder
	var lastWasSpace bool = true // Start with true to trim leading spaces

	for _, r := range s {
		if unicode.IsLetter(r) || unicode.IsDigit(r) {
			result.WriteRune(r)
			lastWasSpace = false
		} else if unicode.IsSpace(r) && !lastWasSpace {
			// Replace any whitespace with a single space
			result.WriteRune(' ')
			lastWasSpace = true
		}
	}

	// Trim trailing space if any
	resultStr := result.String()
	return strings.TrimSpace(resultStr)
}

// levenshteinDistance calculates the Levenshtein distance between two strings
func levenshteinDistance(s1, s2 string) int {
	// Create a matrix of size (len(s1)+1) x (len(s2)+1)
	rows, cols := len(s1)+1, len(s2)+1
	matrix := make([][]int, rows)
	for i := range matrix {
		matrix[i] = make([]int, cols)
	}

	// Initialize the first row and column
	for i := 0; i < rows; i++ {
		matrix[i][0] = i
	}
	for j := 0; j < cols; j++ {
		matrix[0][j] = j
	}

	// Fill the matrix
	for i := 1; i < rows; i++ {
		for j := 1; j < cols; j++ {
			cost := 1
			if s1[i-1] == s2[j-1] {
				cost = 0
			}
			matrix[i][j] = min(
				matrix[i-1][j]+1,      // deletion
				matrix[i][j-1]+1,      // insertion
				matrix[i-1][j-1]+cost, // substitution
			)
		}
	}

	// Return the bottom-right cell
	return matrix[rows-1][cols-1]
}

// min returns the minimum of three integers
func min(a, b, c int) int {
	if a < b {
		if a < c {
			return a
		}
		return c
	}
	if b < c {
		return b
	}
	return c
}

// CompareResponses compares two responses and returns their similarity score
func CompareResponses(response1, response2 string) float64 {
	return CalculateStringSimilarity(response1, response2)
}

// Constants for farming strategy prompts
const FarmingStrategyPrompt = `I have the following portfolio:

%s

Here is the current market price of the tokens in the portfolio:

%s

Here is the current APR of the tokens other than this deposit_apr is 0 and not borrowable:

%s

I want to optimize my yield farming strategy. 

Please recommend a strategy that is delta neutral, meaning you should take both opposite positions between CEX and DEX. The Eisen portfoilio is for DEX, and Binance is for CEX.
In Binance, you can only trade on BTC, ETH, and EIGEN.
In Eisen, you can trade on tokens [USDT, USDC, ETH, WBTC, WETH, cbETH, aBascbETH, aBasweETH, weETH, ezETH, aBasezETH, aBasWETH, aBasUSDC, wstETH, aBaswstETH, aBascbBTC, cbBTC, aBasUSDbC, USDbC] in the portfolio only on Base chain chain id is 8453.
Here is an example of ouput format that should be in JSON format do not print anything else than the JSON. You should only return the JSON:

{
    "exchanges": [
				"binance": {
				    "positions": [
                {
                    "position": "short",
                    "token": "<token_symbol1>",
                    "amount": "<amount>",
                    "price": "<price>",
                    "side": "sell"
                },
                {
                    "position": "short",
                    "token": "<token_symbol2>",
                    "amount": "<amount>",
                    "price": "<price>",
                    "side": "sell"
                }
								....
            ]
				},
				"eisen": {
				    "swaps": [
                {
                    "token_in": "mETH",
                    "token_out": "ETH",
                    "amount": "<amount>",
                },
                {
                    "token_in": "stETH",
                    "token_out": "ETH",
                    "amount": "<amount>",
                }
								....
            ]
				}
    ]
}
`

// StableYieldFarmingAgent is a specialized agent for yield farming strategies
type StableYieldFarmingAgent struct {
	inner Agent
}

// NewStableYieldFarmingAgent creates a new StableYieldFarmingAgent
func NewStableYieldFarmingAgent(agent Agent) *StableYieldFarmingAgent {
	// Set the specialized finance prompt
	agent.SetPrompt("You are a specialized financial advisor focused on stable yield farming strategies. " +
		"Provide conservative, well-researched advice on DeFi protocols, yield optimization, " +
		"risk assessment, and portfolio diversification. Always prioritize security and " +
		"sustainability over high APYs. Include relevant warnings about smart contract risks, " +
		"impermanent loss, and market volatility where appropriate.")

	return &StableYieldFarmingAgent{
		inner: agent,
	}
}

// Chat delegates to the inner agent with the system prompt added
func (a *StableYieldFarmingAgent) Chat(messages []Message) (ChatResponse, error) {
	// Create a new slice with the system prompt as the first message
	allMessages := []Message{
		{
			Role:    "system",
			Content: a.inner.Prompt(),
		},
	}

	// Add the user messages
	allMessages = append(allMessages, messages...)

	// Call the inner agent's chat method
	return a.inner.Chat(allMessages)
}

// GetFarmingStrategy gets a farming strategy recommendation as plain text
func (a *StableYieldFarmingAgent) GetFarmingStrategy(prices string, portfolioSummary string, apr string) (ChatResponse, error) {
	messages := []Message{
		{
			Role:    "user",
			Content: fmt.Sprintf(FarmingStrategyPrompt, portfolioSummary, prices, apr),
		},
	}

	// Get the AI's recommendation as plain text
	return a.Chat(messages)
}

// CompareStrategies compares two farming strategies and returns their similarity score
func (a *StableYieldFarmingAgent) CompareStrategies(strategy1, strategy2 string) float64 {
	return CalculateStringSimilarity(strategy1, strategy2)
}
