curl -i -X POST -d 'email=thomasd_manjvn@hotmail.com&name=Todjmv' http://127.0.0.1:8000/subscriptions

curl --request POST --data 'name=le%20guin&email=ursula_le_guin%40gmail.com'  https://zero2prod-d3zbe.ondigitalocean.app/subscriptions --verbose

DATABASE_URL=postgresql://newsletter:AVNS_bvDDFzB6R06XVe0UEE3@app-54197910-c2ba-4d67-8a5e-80778d8f9588-do-user-24822909-0.e.db.ondigitalocean.com:25060/newsletter?sslmode=require sqlx migrate run

