package handlers

import (
	"Execution_Service/services"
	"encoding/json"
	"log"
	"net/http"
	"os"

	"github.com/gin-gonic/gin"
)

type TaskData struct {
	Price     string `json:"price"`
	Portfolio string `json:"portfolio"`
	Model     string `json:"model"`
	Strategy  string `json:"strategy"`
	Apr       string `json:"apr"`
}

func ExecuteTask(c *gin.Context) {
	log.Println("Executing Task")

	if c.Request.Method == http.MethodPost {
		taskDefinitionId := 0
		price := ""
		portfolio := ""
		model := ""
		apr := ""
		if c.Request.Body != http.NoBody {
			var requestBody map[string]interface{}
			if json.NewDecoder(c.Request.Body).Decode(&requestBody) == nil {
				if val, ok := requestBody["taskDefinitionId"].(int); ok {
					taskDefinitionId = val
				}

				if val, ok := requestBody["price"].(string); ok {
					price = val
				}

				if val, ok := requestBody["portfolio"].(string); ok {
					portfolio = val
				}

				if val, ok := requestBody["model"].(string); ok {
					model = val
				}

				if val, ok := requestBody["apr"].(string); ok {
					apr = val
				}
			}
		}

		log.Printf("taskDefinitionId: %v\n", taskDefinitionId)

		openaiAgent := NewOpenAIAgent(os.Getenv("OPENAI_API_KEY"), model, 0.0)
		agent := NewStableYieldFarmingAgent(openaiAgent)

		agentResponse, err := agent.GetFarmingStrategy(price, portfolio, apr)
		if err != nil {
			log.Println("Error fetching strategy:", err)
			c.JSON(http.StatusInternalServerError, gin.H{
				"error": "Failed to fetch strategy",
			})
		}

		// Create a map to store all the task data
		taskData := TaskData{
			Price:     price,
			Portfolio: portfolio,
			Model:     model,
			Strategy:  agentResponse.Response,
			Apr:       apr,
		}

		// Convert the task data map to JSON
		taskDataJson, err := json.Marshal(taskData)
		if err != nil {
			log.Println("Error marshaling task data to JSON:", err)
			c.JSON(http.StatusInternalServerError, gin.H{
				"error": "Failed to process task data",
			})
			return
		}

		proofOfTask := string(taskDataJson)
		data := ""
		services.SendTask(proofOfTask, data, taskDefinitionId)

		response := services.NewCustomResponse(map[string]interface{}{
			"strategy": agentResponse.Response,
		}, "Task executed successfully")
		c.JSON(http.StatusOK, response)
	} else {
		c.JSON(http.StatusMethodNotAllowed, gin.H{
			"error": "Invalid method",
		})
	}
}
