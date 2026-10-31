namespace FunctionWorld.wit.exports.fluor.fun.v0_2_0;

public class HandlerImpl : IHandler
{
    private static string JsonString(string value)
    {
        return "\"" + value
            .Replace("\\", "\\\\")
            .Replace("\"", "\\\"")
            .Replace("\r", "\\r")
            .Replace("\n", "\\n")
            .Replace("\t", "\\t") + "\"";
    }

    public static IHandler.Response Handle(IHandler.Request request)
    {
        if (request.path != "/hello")
        {
            throw new WitException<IHandler.Error>(
                new IHandler.Error(IHandler.ErrorCode.NOT_FOUND, $"Route not found: {request.path}"),
                0
            );
        }

        if (request.method != "POST")
        {
            throw new WitException<IHandler.Error>(
                new IHandler.Error(IHandler.ErrorCode.BAD_REQUEST, "Only POST is supported in this PoC"),
                0
            );
        }

        var headers = new List<IHandler.Header>
        {
            new("content-type", "application/json"),
            new("x-powered-by", "componentize-dotnet"),
        };

        var query = request.query ?? "";
        var responseBody = $$"""
        {
          "message": "Hello from C#",
          "method": {{JsonString(request.method)}},
          "path": {{JsonString(request.path)}},
          "query": {{JsonString(query)}},
          "body": {{JsonString(request.body)}}
        }
        """;

        return new IHandler.Response(200, headers, responseBody);
    }
}
