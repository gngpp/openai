// main.go
package main

import "C"
import (
	"bufio"
	"encoding/json"
	"fmt"
	"io"
	"net/url"
	"strings"

	http "github.com/bogdanfinn/fhttp"
	ffi "github.com/bogdanfinn/tls-client/cffi_src"
	"github.com/google/uuid"
	cmap "github.com/orcaman/concurrent-map/v2"
)

var bodyMap = cmap.New[*AutoClosingScanner]()

type AutoClosingScanner struct {
	rc   io.ReadCloser
	scan *bufio.Scanner
}

func NewAutoClosingScanner(rc io.ReadCloser) *AutoClosingScanner {
	return &AutoClosingScanner{
		rc:   rc,
		scan: bufio.NewScanner(rc),
	}
}

func (acr *AutoClosingScanner) Scan() bool {
	return acr.scan.Scan()
}

func (acr *AutoClosingScanner) Text() string {
	s := acr.scan.Text()
	if acr.scan.Err() != nil {
		_ = acr.Close()
	}
	return s
}

func (acr *AutoClosingScanner) Close() error {
	return acr.rc.Close()
}

//export DestroyAll
func DestroyAll() {
	ffi.ClearSessionCache()
}

//export DestroySession
func DestroySession(destroySessionParams *C.char) *C.char {
	destroySessionParamsJson := C.GoString(destroySessionParams)

	destroySessionInput := ffi.DestroySessionInput{}
	marshallError := json.Unmarshal([]byte(destroySessionParamsJson), &destroySessionInput)

	if marshallError != nil {
		clientErr := ffi.NewTLSClientError(marshallError)

		return handleErrorResponse("", false, clientErr)
	}

	ffi.RemoveSession(destroySessionInput.SessionId)

	out := ffi.DestroyOutput{
		Id:      uuid.New().String(),
		Success: true,
	}

	jsonResponse, marshallError := json.Marshal(out)

	if marshallError != nil {
		clientErr := ffi.NewTLSClientError(marshallError)

		return handleErrorResponse(destroySessionInput.SessionId, true, clientErr)
	}

	return C.CString(string(jsonResponse))
}

//export GetCookiesFromSession
func GetCookiesFromSession(getCookiesParams *C.char) *C.char {
	getCookiesParamsJson := C.GoString(getCookiesParams)

	cookiesInput := ffi.GetCookiesFromSessionInput{}
	marshallError := json.Unmarshal([]byte(getCookiesParamsJson), &cookiesInput)

	if marshallError != nil {
		clientErr := ffi.NewTLSClientError(marshallError)

		return handleErrorResponse("", false, clientErr)
	}

	tlsClient, err := ffi.GetClient(cookiesInput.SessionId)

	if err != nil {
		clientErr := ffi.NewTLSClientError(err)

		return handleErrorResponse(cookiesInput.SessionId, true, clientErr)
	}

	u, parsErr := url.Parse(cookiesInput.Url)
	if parsErr != nil {
		clientErr := ffi.NewTLSClientError(parsErr)

		return handleErrorResponse(cookiesInput.SessionId, true, clientErr)
	}

	cookies := tlsClient.GetCookies(u)

	out := ffi.CookiesFromSessionOutput{
		Id:      uuid.New().String(),
		Cookies: cookies,
	}

	jsonResponse, marshallError := json.Marshal(out)

	if marshallError != nil {
		clientErr := ffi.NewTLSClientError(marshallError)

		return handleErrorResponse(cookiesInput.SessionId, true, clientErr)
	}

	return C.CString(string(jsonResponse))
}

//export AddCookiesToSession
func AddCookiesToSession(addCookiesParams *C.char) *C.char {
	addCookiesParamsJson := C.GoString(addCookiesParams)

	cookiesInput := ffi.AddCookiesToSessionInput{}
	marshallError := json.Unmarshal([]byte(addCookiesParamsJson), &cookiesInput)

	if marshallError != nil {
		clientErr := ffi.NewTLSClientError(marshallError)

		return handleErrorResponse("", false, clientErr)
	}

	tlsClient, err := ffi.GetClient(cookiesInput.SessionId)

	if err != nil {
		clientErr := ffi.NewTLSClientError(err)

		return handleErrorResponse(cookiesInput.SessionId, true, clientErr)
	}

	u, parsErr := url.Parse(cookiesInput.Url)
	if parsErr != nil {
		clientErr := ffi.NewTLSClientError(parsErr)

		return handleErrorResponse(cookiesInput.SessionId, true, clientErr)
	}

	tlsClient.SetCookies(u, cookiesInput.Cookies)

	allCookies := tlsClient.GetCookies(u)

	out := ffi.CookiesFromSessionOutput{
		Id:      uuid.New().String(),
		Cookies: allCookies,
	}

	jsonResponse, marshallError := json.Marshal(out)

	if marshallError != nil {
		clientErr := ffi.NewTLSClientError(marshallError)

		return handleErrorResponse(cookiesInput.SessionId, true, clientErr)
	}

	return C.CString(string(jsonResponse))
}

//export RequestStream
func RequestStream(requestParams *C.char) *C.char {
	requestParamsJson := C.GoString(requestParams)

	requestInput := ffi.RequestInput{}
	marshallError := json.Unmarshal([]byte(requestParamsJson), &requestInput)

	if marshallError != nil {
		clientErr := ffi.NewTLSClientError(marshallError)

		return handleErrorResponse("", false, clientErr)
	}

	tlsClient, sessionId, withSession, err := ffi.CreateClient(requestInput)

	if err != nil {
		return handleErrorResponse(sessionId, withSession, err)
	}

	req, err := ffi.BuildRequest(requestInput)

	if err != nil {
		clientErr := ffi.NewTLSClientError(err)

		return handleErrorResponse(sessionId, withSession, clientErr)
	}

	cookies := buildCookies(requestInput.RequestCookies)

	if len(cookies) > 0 {
		tlsClient.SetCookies(req.URL, cookies)
	}

	resp, reqErr := tlsClient.Do(req)

	if reqErr != nil {
		clientErr := ffi.NewTLSClientError(fmt.Errorf("failed to do request: %w", reqErr))

		return handleErrorResponse(sessionId, withSession, clientErr)
	}

	if resp == nil {
		clientErr := ffi.NewTLSClientError(fmt.Errorf("response is nil"))

		return handleErrorResponse(sessionId, withSession, clientErr)
	}

	targetCookies := tlsClient.GetCookies(resp.Request.URL)

	response, err := BuildStreamResponse(sessionId, withSession, resp, targetCookies, requestInput)
	if err != nil {
		return handleErrorResponse(sessionId, withSession, err)
	}

	jsonResponse, marshallError := json.Marshal(response)

	if marshallError != nil {
		clientErr := ffi.NewTLSClientError(marshallError)

		return handleErrorResponse(sessionId, withSession, clientErr)
	}
	return C.CString(string(jsonResponse))
}

//export NextStreamLine
func NextStreamLine(id *C.char) *C.char {
	requestId := C.GoString(id)
	next, ok := bodyMap.Get(requestId)
	if ok {
		for next.Scan() {
			line := next.Text()
			if strings.HasPrefix(line, "data: [DONE]") {
				_ = next.Close()
				bodyMap.Remove(requestId)
			} else if strings.HasPrefix(line, "data: ") {
				return C.CString(line)
			}
		}
	}
	return C.CString("data: [DONE]")
}

//export Request
func Request(requestParams *C.char) *C.char {
	requestParamsJson := C.GoString(requestParams)

	requestInput := ffi.RequestInput{}
	marshallError := json.Unmarshal([]byte(requestParamsJson), &requestInput)

	if marshallError != nil {
		clientErr := ffi.NewTLSClientError(marshallError)

		return handleErrorResponse("", false, clientErr)
	}

	tlsClient, sessionId, withSession, err := ffi.CreateClient(requestInput)

	if err != nil {
		return handleErrorResponse(sessionId, withSession, err)
	}

	req, err := ffi.BuildRequest(requestInput)

	if err != nil {
		clientErr := ffi.NewTLSClientError(err)

		return handleErrorResponse(sessionId, withSession, clientErr)
	}

	cookies := buildCookies(requestInput.RequestCookies)

	if len(cookies) > 0 {
		tlsClient.SetCookies(req.URL, cookies)
	}

	resp, reqErr := tlsClient.Do(req)

	if reqErr != nil {
		clientErr := ffi.NewTLSClientError(fmt.Errorf("failed to do request: %w", reqErr))

		return handleErrorResponse(sessionId, withSession, clientErr)
	}

	if resp == nil {
		clientErr := ffi.NewTLSClientError(fmt.Errorf("response is nil"))

		return handleErrorResponse(sessionId, withSession, clientErr)
	}

	targetCookies := tlsClient.GetCookies(resp.Request.URL)

	response, err := ffi.BuildResponse(sessionId, withSession, resp, targetCookies, requestInput)
	if err != nil {
		return handleErrorResponse(sessionId, withSession, err)
	}

	jsonResponse, marshallError := json.Marshal(response)

	if marshallError != nil {
		clientErr := ffi.NewTLSClientError(marshallError)

		return handleErrorResponse(sessionId, withSession, clientErr)
	}
	return C.CString(string(jsonResponse))
}

func handleErrorResponse(sessionId string, withSession bool, err *ffi.TLSClientError) *C.char {
	response := ffi.Response{
		Id:      uuid.New().String(),
		Status:  0,
		Body:    err.Error(),
		Headers: nil,
		Cookies: nil,
	}

	if withSession {
		response.SessionId = sessionId
	}

	jsonResponse, marshallError := json.Marshal(response)

	if marshallError != nil {
		errStr := C.CString(marshallError.Error())

		return errStr
	}

	return C.CString(string(jsonResponse))
}

// BuildResponse constructs a client response from a given HTTP response. The client response can then be sent to the interface consumer.
func BuildStreamResponse(sessionId string, withSession bool, resp *http.Response, cookies []*http.Cookie, input ffi.RequestInput) (ffi.Response, *ffi.TLSClientError) {

	uuid := uuid.New().String()

	bodyMap.SetIfAbsent(uuid, NewAutoClosingScanner(resp.Body))

	response := ffi.Response{
		Id:           uuid,
		Status:       resp.StatusCode,
		UsedProtocol: resp.Proto,
		Body:         "",
		Headers:      resp.Header,
		Target:       "",
		Cookies:      cookiesToMap(cookies),
	}

	if resp.Request != nil && resp.Request.URL != nil {
		response.Target = resp.Request.URL.String()
	}

	if withSession {
		response.SessionId = sessionId
	}

	return response, nil
}

func buildCookies(cookies []ffi.CookieInput) []*http.Cookie {
	var ret []*http.Cookie

	for _, cookie := range cookies {
		ret = append(ret, &http.Cookie{
			Name:    cookie.Name,
			Value:   cookie.Value,
			Path:    cookie.Path,
			Domain:  cookie.Domain,
			Expires: cookie.Expires.Time,
		})
	}

	return ret
}

func cookiesToMap(cookies []*http.Cookie) map[string]string {
	ret := make(map[string]string, 0)

	for _, c := range cookies {
		ret[c.Name] = c.Value
	}

	return ret
}

func main() {

}
